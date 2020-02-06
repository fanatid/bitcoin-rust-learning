use std::collections::LinkedList;

use crate::server::bitcoind;

use bitcoind::rpc::ResponseBlockHeader;

#[derive(Debug)]
pub struct Block {
    pub height: u32,
    pub hash: String,
    pub prevhash: Option<String>,
}

impl From<ResponseBlockHeader> for Block {
    fn from(header: ResponseBlockHeader) -> Self {
        Block {
            height: header.height,
            hash: header.hash.clone(),
            prevhash: header.previousblockhash,
        }
    }
}

#[derive(Debug)]
pub enum BlockError {
    HeightMismatch,
    HashMismatch,
}

#[derive(Debug, Default)]
pub struct AppState {
    blocks: LinkedList<Block>,
}

impl AppState {
    pub fn reset(&mut self) {
        self.blocks.clear()
    }

    pub fn blocks_len(&self) -> usize {
        self.blocks.len()
    }

    pub fn blocks_first(&self) -> Option<&Block> {
        self.blocks.front()
    }

    pub fn blocks_last(&self) -> Option<&Block> {
        self.blocks.back()
    }

    pub fn blocks_push_front(&mut self, block: Block) -> Result<&Block, BlockError> {
        if let Some(front) = self.blocks_first() {
            if block.height + 1 != front.height {
                return Err(BlockError::HeightMismatch);
            }
            if front.prevhash.is_none() || &block.hash != front.prevhash.as_ref().unwrap() {
                return Err(BlockError::HashMismatch);
            }
        }

        self.blocks.push_front(block);
        Ok(self.blocks_first().unwrap()) // How return just &block ?
    }

    pub fn blocks_push_back(&mut self, block: Block) -> Result<&Block, BlockError> {
        if let Some(back) = self.blocks_last() {
            if block.height - 1 != back.height {
                return Err(BlockError::HeightMismatch);
            }
            if block.prevhash.is_none() || block.prevhash.as_ref().unwrap() != &back.hash {
                return Err(BlockError::HashMismatch);
            }
        }

        self.blocks.push_back(block);
        Ok(self.blocks_last().unwrap())
    }

    pub fn blocks_pop_first(&mut self) {
        self.blocks.pop_front();
    }

    pub fn blocks_pop_last(&mut self) {
        self.blocks.pop_back();
    }
}
