use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use actix_web::http::Uri;
use awc::{ClientBuilder, FrozenClientRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;

// Only RPC client.
// Right way is create module `bitcoind` and implement RPC, Rest, ZMQ.
pub struct RPCClient {
    client: FrozenClientRequest,
    nonce: Arc<Mutex<u64>>,
}

impl RPCClient {
    // Construct new RPCClient for specified URL
    pub fn new(url: &str) -> RPCClient {
        let (url, username, password) = RPCClient::parse_url(url);

        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(60))
            .disable_redirects()
            .basic_auth(username, password.as_deref())
            .header("Content-Type", "application/json")
            .finish()
            .post(url)
            .freeze()
            .unwrap();

        RPCClient {
            client,
            nonce: Arc::new(Mutex::new(0)),
        }
    }

    // Prase given URL with username/password
    fn parse_url(url: &str) -> (String, String, Option<String>) {
        let uri: Uri = url.parse().unwrap();
        let authority = uri.authority().unwrap();

        let mut url = format!("{}://{}", uri.scheme_str().unwrap(), authority.host());
        if let Some(port) = authority.port() {
            url = format!("{}:{}", url, port.as_str());
        }

        let auth: Vec<&str> = authority
            .as_str()
            .split('@')
            .next()
            .expect("Auth data not found in URL to bitcoind")
            .split(':')
            .collect();

        match auth.len() {
            1 => (url, auth[0].to_owned(), None),
            2 => (url, auth[0].to_owned(), Some(auth[1].to_owned())),
            _ => panic!("Invalud auth data in URL to bitcoind: {}", auth.join(":")),
        }
    }

    async fn request<T>(&self, body: serde_json::value::Value) -> Response<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut res = self.client.send_body(body).await.unwrap();

        // We ignore status, because expect error information in the body
        // let status = res.status();
        // if status.as_u16() != 200 {
        //     let message = match status.canonical_reason() {
        //         Some(reason) => format!(" ({})", reason),
        //         None => "".to_owned(),
        //     };
        //     panic!("Bitcoind RPC Client, Status code: {}{} is not OK", status.as_u16(), message);
        // }

        // Change response body limit to 256 MiB
        // This require store all response and parsed result, what is shitty
        // Should be serde_json::from_reader
        let body = res.body().limit(256 * 1024 * 1024).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    async fn call<T>(&self, method: &str, params: Option<&[serde_json::Value]>) -> Response<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let body = {
            let mut nonce = self.nonce.lock().unwrap();
            *nonce = nonce.wrapping_add(1);
            json!(Request {
                method,
                params,
                id: *nonce
            })
        };

        self.request::<T>(body).await
    }

    pub async fn getblockchaininfo(&self) -> ResponseBlockchainInfo {
        let res = self.call("getblockchaininfo", None).await;
        if let Some(err) = res.error {
            panic!("{}", err);
        }

        res.result.unwrap()
    }

    pub async fn getblockheader(&self, hash: &str) -> Option<ResponseBlockHeader> {
        let params = [hash.into(), true.into()];
        let res = self.call("getblockheader", Some(&params)).await;
        res.result
    }
}

#[derive(Debug, Serialize)]
pub struct Request<'a, 'b> {
    pub method: &'a str,
    pub params: Option<&'b [serde_json::Value]>,
    pub id: u64,
}

#[derive(Debug, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RPC Error ({}): {}", self.code, self.message)
    }
}

#[derive(Debug, Deserialize)]
pub struct Response<T> {
    pub result: Option<T>,
    pub error: Option<RpcError>,
    pub id: u64,
}

#[derive(Debug, Deserialize)]
pub struct ResponseBlockchainInfo {
    pub chain: String,
    pub blocks: u32,
    pub bestblockhash: String,
}

#[derive(Debug, Deserialize)]
pub struct ResponseBlockHeader {
    pub hash: String,
    pub height: u32,
    pub previousblockhash: Option<String>,
    pub nextblockhash: Option<String>,
}
