use std::error::Error;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use awc::error::FreezeRequestError;
use awc::{ClientBuilder, FrozenClientRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;

#[derive(Debug)]
pub enum RPCClientError {
    InvalidUrl(url::ParseError),
    InvalidUrlScheme(String),
    InvalidUrlFreeze(FreezeRequestError),
}

impl fmt::Display for RPCClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::InvalidUrl(ref e) => e.fmt(f),
            Self::InvalidUrlScheme(ref s) => s.fmt(f),
            Self::InvalidUrlFreeze(ref e) => e.fmt(f),
        }
    }
}

impl Error for RPCClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            Self::InvalidUrl(ref e) => Some(e),
            Self::InvalidUrlScheme(_) => None,
            Self::InvalidUrlFreeze(_) => None,
        }
    }
}

pub struct RPCClient {
    client: FrozenClientRequest,
    nonce: Arc<Mutex<u64>>,
}

impl RPCClient {
    // Construct new RPCClient for specified URL
    pub fn new(url: &str) -> Result<RPCClient, RPCClientError> {
        let (url, username, password) = RPCClient::parse_url(url)?;

        let mut client = ClientBuilder::new()
            .timeout(Duration::from_secs(60))
            .disable_redirects()
            .header("Content-Type", "application/json");
        if !username.is_empty() {
            client = client.basic_auth(username, password.as_deref());
        }

        let frozen_client = client
            .finish()
            .post(url)
            .freeze()
            .map_err(RPCClientError::InvalidUrlFreeze)?;

        Ok(RPCClient {
            client: frozen_client,
            nonce: Arc::new(Mutex::new(0)),
        })
    }

    // Prase given URL with username/password
    fn parse_url(url: &str) -> Result<(String, String, Option<String>), RPCClientError> {
        let mut parsed = Url::parse(url).map_err(RPCClientError::InvalidUrl)?;
        match parsed.scheme() {
            "http" | "https" => {}
            scheme => {
                let msg = format!(r#"Invalid scheme in bitcoind URL: "{}" ({})"#, scheme, url);
                return Err(RPCClientError::InvalidUrlScheme(msg));
            }
        }

        let username = parsed.username().to_owned();
        let password = parsed.password().map(|s| s.to_owned());

        // Return Err only if `.cannot_be_a_base` is true
        // Since we already verified that scheme is http/https, unwrap is safe
        parsed.set_username("").unwrap();
        parsed.set_password(None).unwrap();

        Ok((parsed.into_string(), username, password))
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
pub struct Response<T> {
    pub result: Option<T>,
    pub error: Option<ResponseError>,
    pub id: u64,
}

#[derive(Debug, Deserialize)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Bitcoind RPC error ({}): {}", self.code, self.message)
    }
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
