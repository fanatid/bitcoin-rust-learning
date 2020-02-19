use serde::Serialize;

use super::bitcoind::json::ResponseBlock;

#[derive(Debug, Serialize)]
pub struct Transaction {
    pub hash: String,
    pub size: u32,
}

#[derive(Debug, Serialize)]
pub struct Block {
    pub height: u32,
    pub hash: String,
    pub size: u32,
    pub transactions: Vec<Transaction>,
}

impl From<ResponseBlock> for Block {
    fn from(block: ResponseBlock) -> Self {
        Block {
            height: block.height,
            hash: block.hash,
            size: block.size,
            transactions: block
                .transactions
                .into_iter()
                .map(|tx| Transaction {
                    hash: tx.hash,
                    size: tx.size,
                })
                .collect(),
        }
    }
}
