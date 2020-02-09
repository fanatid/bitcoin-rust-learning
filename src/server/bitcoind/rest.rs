// We use REST interfance, because extract block blocks through RPC was slow,
// not all coin clients fixed it.
// See issue in bitcoin repo: https://github.com/bitcoin/bitcoin/issues/15925

use std::time::Duration;

use awc::{Client, ClientBuilder};
use derivative::Derivative;
use url::Url;

use super::json::*;
use super::BitcoindError;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct RESTClient {
    #[derivative(Debug = "ignore")]
    client: Client,
    url: Url,
}

impl RESTClient {
    pub fn new(url: &str) -> RESTClient {
        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .disable_redirects()
            .header("Content-Type", "application/json");

        RESTClient {
            client: client.finish(),
            url: Url::parse(url).unwrap(), // url already verified above
        }
    }

    pub async fn getblockchaininfo(&mut self) -> Result<ResponseBlockchainInfo, BitcoindError> {
        self.url.set_path("rest/chaininfo.json");
        let timeout = Duration::from_millis(200);

        let res_fut = self.client.get(self.url.as_ref()).timeout(timeout).send();
        let mut res = res_fut.await.map_err(BitcoindError::SendRequest)?;

        let body = res.body().await.map_err(BitcoindError::ResponsePayload)?;

        match res.status().as_u16() {
            200 => serde_json::from_slice(&body).map_err(BitcoindError::ResponseParse),
            code => {
                let msg = String::from_utf8_lossy(&body).trim().to_owned();
                Err(BitcoindError::ResultRest(code, msg))
            }
        }
    }

    pub async fn getblock(&mut self, hash: &str) -> Result<Option<ResponseBlock>, BitcoindError> {
        self.url.set_path(&format!("rest/block/{}.json", hash));
        let res_fut = self.client.get(self.url.as_ref()).send();
        let mut res = res_fut.await.map_err(BitcoindError::SendRequest)?;

        let status_code = res.status().as_u16();
        if status_code == 404 {
            return Ok(None);
        }

        // Change response body limit to 256 MiB
        // This require store all response and parsed result, what is shitty
        // Should be serde_json::from_reader
        let body_fut = res.body().limit(256 * 1024 * 1024);
        let body = body_fut.await.map_err(BitcoindError::ResponsePayload)?;
        if status_code != 200 {
            let msg = String::from_utf8_lossy(&body).trim().to_owned();
            return Err(BitcoindError::ResultRest(status_code, msg));
        }

        let parsed = serde_json::from_slice(&body);
        let block: ResponseBlock = parsed.map_err(BitcoindError::ResponseParse)?;
        if block.hash != hash {
            return Err(BitcoindError::ResultMismatch);
        }

        Ok(Some(block))
    }
}
