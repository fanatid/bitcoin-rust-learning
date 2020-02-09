use std::time::Duration;

use awc::{Client, ClientBuilder};
use derivative::Derivative;
use serde_json::json;

use super::json::*;
use super::BitcoindError;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct RPCClient {
    #[derivative(Debug = "ignore")]
    client: Client,
    url: String,
    req_id: u64,
}

impl RPCClient {
    // Construct new RPCClient for specified URL
    pub fn new(url: &str, username: &str, password: Option<&str>) -> RPCClient {
        let mut client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .disable_redirects()
            .header("Content-Type", "application/json");
        if !username.is_empty() {
            client = client.basic_auth(username, password);
        }

        RPCClient {
            client: client.finish(),
            url: url.to_owned(),
            req_id: 0,
        }
    }

    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        body: serde_json::value::Value,
    ) -> Result<Response<T>, BitcoindError> {
        let res_fut = self.client.post(&self.url).send_body(body);
        let mut res = res_fut.await.map_err(BitcoindError::SendRequest)?;

        // We ignore status, because expect error information in the body
        // let status = res.status();

        // Change response body limit to 10 MiB
        // This require store all response and parsed result, what is shitty
        // Should be serde_json::from_reader
        let body_fut = res.body().limit(10 * 1024 * 1024);
        let body = body_fut.await.map_err(BitcoindError::ResponsePayload)?;
        serde_json::from_slice(&body).map_err(BitcoindError::ResponseParse)
    }

    async fn call<T: serde::de::DeserializeOwned>(
        &mut self,
        method: &str,
        params: Option<&[serde_json::Value]>,
    ) -> Result<T, BitcoindError> {
        let req_id = self.req_id;
        self.req_id = self.req_id.wrapping_add(1);

        let body = json!(Request {
            method,
            params,
            id: req_id,
        });

        let data = self.request::<T>(body).await?;
        if data.id != req_id {
            return Err(BitcoindError::NonceMismatch);
        }
        if let Some(error) = data.error {
            return Err(BitcoindError::ResultRPC(error));
        }
        match data.result {
            None => Err(BitcoindError::ResultNotFound),
            Some(result) => Ok(result),
        }
    }

    pub async fn getblockchaininfo(&mut self) -> Result<ResponseBlockchainInfo, BitcoindError> {
        self.call("getblockchaininfo", None).await
    }

    pub async fn getblockhash(&mut self, height: u32) -> Result<Option<String>, BitcoindError> {
        let params = [height.into()];
        match self.call::<String>("getblockhash", Some(&params)).await {
            Ok(hash) => Ok(Some(hash)),
            Err(BitcoindError::ResultRPC(error)) => {
                // Block height out of range
                if error.code == -8 {
                    Ok(None)
                } else {
                    Err(BitcoindError::ResultRPC(error))
                }
            }
            Err(error) => Err(error),
        }
    }
}
