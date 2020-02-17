// We use REST interfance, because extract block blocks through RPC was slow,
// not all coin clients fixed it.
// See issue in bitcoin repo: https://github.com/bitcoin/bitcoin/issues/15925

use std::fmt;
use std::time::Duration;

use reqwest::{header, redirect, Client, ClientBuilder};
use url::Url;

use super::{json::*, BitcoindError, BitcoindResult};

pub struct RESTClient {
    client: Client,
    url: Url,
}

impl fmt::Debug for RESTClient {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("RESTClient")
            .field("url", &self.url)
            .finish()
    }
}

impl RESTClient {
    pub fn new(url: Url) -> BitcoindResult<RESTClient> {
        let mut headers = header::HeaderMap::with_capacity(1);
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("applicaiton/json"),
        );

        let client = ClientBuilder::new()
            .connect_timeout(Duration::from_millis(100))
            .timeout(Duration::from_secs(30))
            .default_headers(headers)
            .no_gzip()
            .redirect(redirect::Policy::none());

        Ok(RESTClient {
            client: client.build().map_err(BitcoindError::Reqwest)?,
            url,
        })
    }

    pub async fn getblockchaininfo(&mut self) -> BitcoindResult<ResponseBlockchainInfo> {
        self.url.set_path("rest/chaininfo.json");
        let timeout = Duration::from_millis(200);

        let res_fut = self.client.get(self.url.clone()).timeout(timeout).send();
        let res = res_fut.await.map_err(BitcoindError::Reqwest)?;
        let status_code = res.status().as_u16();

        let body = res.bytes().await.map_err(BitcoindError::Reqwest)?;

        match status_code {
            200 => serde_json::from_slice(&body).map_err(BitcoindError::ResponseParse),
            code => {
                let msg = String::from_utf8_lossy(&body).trim().to_owned();
                Err(BitcoindError::ResultRest(code, msg))
            }
        }
    }

    pub async fn getblock(&mut self, hash: &str) -> BitcoindResult<Option<ResponseBlock>> {
        self.url.set_path(&format!("rest/block/{}.json", hash));
        let res_fut = self.client.get(self.url.clone()).send();
        let res = res_fut.await.map_err(BitcoindError::Reqwest)?;

        let status_code = res.status().as_u16();
        if status_code == 404 {
            return Ok(None);
        }

        // Should be serde_json::from_reader
        let body_fut = res.bytes();
        let body = body_fut.await.map_err(BitcoindError::Reqwest)?;
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
