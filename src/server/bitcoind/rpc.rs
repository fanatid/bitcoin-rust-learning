use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use awc::{ClientBuilder, FrozenClientRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;
use url::Url;

#[derive(Debug)]
pub enum RPCClientError {
    InvalidUrl(url::ParseError),
    InvalidUrlScheme(String, String),
    InvalidUrlFreeze(awc::error::FreezeRequestError),
    RequestSend(awc::error::SendRequestError),
    ResponsePayload(awc::error::PayloadError),
    ResponseParse(serde_json::Error),
    ResponseInvalidNonce(String),
    ResponseResultError(ResponseError),
    ResponseResultNotFound(String),
}

impl fmt::Display for RPCClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use RPCClientError::*;
        match *self {
            InvalidUrl(ref e) => e.fmt(f),
            InvalidUrlScheme(ref scheme, ref url) => write!(
                f,
                r#"Invalid scheme in bitcoind URL: "{}" ({})"#,
                scheme, url
            ),
            InvalidUrlFreeze(ref e) => e.fmt(f),
            RequestSend(ref e) => e.fmt(f),
            ResponsePayload(ref e) => e.fmt(f),
            ResponseParse(ref e) => e.fmt(f),
            ResponseInvalidNonce(ref method) => write!(f, r#"Invalid nonce for: "{}""#, method),
            ResponseResultError(ref e) => e.fmt(f),
            ResponseResultNotFound(ref method) => {
                write!(f, r#"Not found result for: "{}""#, method)
            }
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
                return Err(RPCClientError::InvalidUrlScheme(
                    scheme.to_owned(),
                    url.to_owned(),
                ))
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

    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        body: serde_json::value::Value,
    ) -> Result<Response<T>, RPCClientError> {
        let res_fut = self.client.send_body(body);
        let mut res = res_fut.await.map_err(RPCClientError::RequestSend)?;

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
        let body_fut = res.body().limit(256 * 1024 * 1024);
        let body = body_fut.await.map_err(RPCClientError::ResponsePayload)?;
        serde_json::from_slice(&body).map_err(RPCClientError::ResponseParse)
    }

    async fn call<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Option<&[serde_json::Value]>,
    ) -> Result<T, RPCClientError> {
        let nonce = {
            let mut nonce = self.nonce.lock().unwrap();
            *nonce = nonce.wrapping_add(1);
            *nonce
        };
        let body = json!(Request {
            method,
            params,
            id: nonce,
        });

        let data = self.request::<T>(body).await?;
        if data.id != nonce {
            return Err(RPCClientError::ResponseInvalidNonce(method.to_owned()));
        }
        if let Some(error) = data.error {
            return Err(RPCClientError::ResponseResultError(error));
        }
        match data.result {
            None => Err(RPCClientError::ResponseResultNotFound(method.to_owned())),
            Some(result) => Ok(result),
        }
    }

    pub async fn getblockchaininfo(&self) -> Result<ResponseBlockchainInfo, RPCClientError> {
        self.call("getblockchaininfo", None).await
    }

    pub async fn getblockheader(&self, hash: &str) -> Result<ResponseBlockHeader, RPCClientError> {
        let params = [hash.into(), true.into()];
        self.call("getblockheader", Some(&params)).await
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
    pub id: u64,
    pub error: Option<ResponseError>,
    pub result: Option<T>,
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
