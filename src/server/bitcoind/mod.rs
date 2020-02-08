use url::Url;

pub use self::error::BitcoindError;
use self::json::*;
use self::rest::RESTClient;
use self::rpc::RPCClient;

mod error;
pub mod json;
mod rest;
mod rpc;

type BitcoindResult<T> = Result<T, BitcoindError>;

#[derive(Debug)]
pub struct Bitcoind {
    rest: RESTClient,
    rpc: RPCClient,
}

impl Bitcoind {
    pub fn new(url: &str) -> BitcoindResult<Bitcoind> {
        let (url, username, password) = Self::parse_url(url)?;

        Ok(Bitcoind {
            rest: RESTClient::new(&url),
            rpc: RPCClient::new(&url, &username, password.as_deref()),
        })
    }

    // Prase given URL with username/password
    fn parse_url(url: &str) -> BitcoindResult<(String, String, Option<String>)> {
        let mut parsed = Url::parse(url).map_err(BitcoindError::InvalidUrl)?;
        match parsed.scheme() {
            "http" | "https" => {}
            scheme => return Err(BitcoindError::InvalidUrlScheme(scheme.to_owned())),
        }

        let username = parsed.username().to_owned();
        let password = parsed.password().map(|s| s.to_owned());

        // Return Err only if `.cannot_be_a_base` is true
        // Since we already verified that scheme is http/https, unwrap is safe
        parsed.set_username("").unwrap();
        parsed.set_password(None).unwrap();

        Ok((parsed.into_string(), username, password))
    }

    pub async fn getblockchaininfo(&mut self) -> BitcoindResult<ResponseBlockchainInfo> {
        self.rpc.getblockchaininfo().await
    }

    pub async fn getblockbyheight(&mut self, height: u32) -> BitcoindResult<Option<ResponseBlock>> {
        let hash = self.rpc.getblockhash(height).await?;
        match hash {
            Some(hash) => match self.getblockbyhash(&hash).await? {
                Some(block) => {
                    if block.height != height {
                        Err(BitcoindError::ResultMismatch)
                    } else {
                        Ok(Some(block))
                    }
                }
                None => Ok(None),
            },
            None => Ok(None),
        }
    }

    // TODO: use some &Number256 instead &str
    pub async fn getblockbyhash(&mut self, hash: &str) -> BitcoindResult<Option<ResponseBlock>> {
        self.rest.getblock(hash).await
    }
}
