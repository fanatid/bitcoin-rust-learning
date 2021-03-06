use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

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
        write!(
            f,
            "Bitcoind RPC error (code: {}): {}",
            self.code, self.message
        )
    }
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct ResponseBlockchainInfo {
    pub chain: String,
    pub blocks: u32,
    pub bestblockhash: String,
}

#[derive(Debug, Deserialize)]
pub struct ResponseBlock {
    pub hash: String,
    pub height: u32,
    pub previousblockhash: Option<String>,
    pub size: u32,
    #[serde(rename = "tx")]
    pub transactions: Vec<ResponseBlockTransaction>,
}

#[derive(Debug, Deserialize)]
pub struct ResponseBlockTransaction {
    pub hash: String,
    pub size: u32,
}

pub type ResponseRawMempool = HashMap<String, ResponseRawMempoolTransaction>;

#[derive(Debug, Deserialize)]
pub struct ResponseRawMempoolTransaction {
    #[serde(rename = "vsize")]
    pub size: u32,
}
