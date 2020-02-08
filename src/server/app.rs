use std::collections::LinkedList;
use std::time::{Duration, SystemTime};

use log::info;

use super::bitcoind::Bitcoind;
use super::error::AppError;

use super::bitcoind::json::ResponseBlock;

const APP_BLOCKS_MINIMUM: usize = 6;
const UPDATE_DELAY_MAX: Duration = Duration::from_micros(25);
const UPDATE_DELAY_MIN: Duration = Duration::from_micros(10);

#[derive(Debug)]
pub struct App {
    bitcoind: Bitcoind,

    blocks: LinkedList<Block>,
}

impl App {
    pub fn new(bitcoind: Bitcoind) -> App {
        App {
            bitcoind,
            blocks: LinkedList::new(),
        }
    }

    pub async fn run(mut app: App) -> Result<(), AppError> {
        // TODO: app.bitcoind.init() -- validate bitcoind from rpc & rest
        app.init_blocks().await?;

        loop {
            // Save current timestamp for timeout after check
            let ts = SystemTime::now();

            // Update our chain
            let require_delay = app.update_blocks().await?;
            if !require_delay {
                continue;
            }

            // UPDATE_DELAY_MAX between calls, but minimum UPDATE_DELAY_MIN
            let elapsed = ts.elapsed().unwrap();
            let sleep_duration = match UPDATE_DELAY_MAX.checked_sub(elapsed) {
                Some(delay) => std::cmp::max(delay, UPDATE_DELAY_MIN),
                None => UPDATE_DELAY_MIN,
            };
            actix_rt::time::delay_for(sleep_duration).await;
        }

        // Ok(())
    }

    // Initialize our chain
    async fn init_blocks(&mut self) -> Result<(), AppError> {
        // Keep at least 6 blocks in chain
        while self.blocks.len() < APP_BLOCKS_MINIMUM {
            // Get prevhash from first known block or just get tip
            let hash = if let Some(block) = self.blocks.front() {
                match block.prevhash {
                    None => return Err(AppError::NotEnoughBlocks),
                    Some(ref hash) => hash.clone(),
                }
            } else {
                let info = self.bitcoind.getblockchaininfo().await;
                info.map_err(AppError::Bitcoind)?.bestblockhash
            };

            // Try fetch block
            let block_fut = self.bitcoind.getblockbyhash(&hash);
            let block = block_fut.await.map_err(AppError::Bitcoind)?;

            // If block not found, try again if there is no blocks, otherwise blockchain corrupted
            if block.is_none() {
                if self.blocks.is_empty() {
                    continue;
                } else {
                    return Err(AppError::InvalidBlockchain);
                }
            };

            // Check that chain is valid
            let block = Block::from(block.unwrap());
            if let Some(front) = self.blocks.front() {
                if block.height + 1 != front.height {
                    return Err(AppError::InvalidBlockchain);
                }
                if front.prevhash.is_none() || &block.hash != front.prevhash.as_ref().unwrap() {
                    return Err(AppError::InvalidBlockchain);
                }
            }

            // Add block
            self.blocks.push_front(block);
            let block = self.blocks.front().unwrap();
            info!("Add block {}: {}", block.height, &block.hash);
        }

        Ok(())
    }

    // Update our chain, return `true` if need call update again
    async fn update_blocks(&mut self) -> Result<bool, AppError> {
        // We always keep blocks, so unwrap is safe
        let mut last = self.blocks.back().unwrap();

        // Get bitcoind info
        let info_fut = self.bitcoind.getblockchaininfo();
        let info = info_fut.await.map_err(AppError::Bitcoind)?;

        // Best hash did not changed, return
        if info.bestblockhash == last.hash {
            return Ok(false);
        }

        // Remove blocks in our chain on reorg
        while last.height >= info.blocks {
            self.blocks.pop_back();
            self.init_blocks().await?;
            last = self.blocks.back().unwrap();
        }

        // Add maximum 1 block
        let block_fut = self.bitcoind.getblockbyheight(last.height + 1);
        match block_fut.await.map_err(AppError::Bitcoind)? {
            Some(block) => {
                let block = Block::from(block);

                // If next block do not have previous blockhash, something wrong with blockchain
                if block.prevhash.is_none() {
                    return Err(AppError::InvalidBlockchain);
                }

                // If previoush hash match to our best hash in new block, add it
                if block.prevhash.as_ref().unwrap() == &last.hash {
                    self.blocks.pop_front();
                    self.blocks.push_back(block);
                    let block = self.blocks.back().unwrap();
                    info!("Add block {}: {}", block.height, &block.hash);
                } else {
                    // If previous block hash did not match, remove best block
                    self.blocks.pop_back();
                    self.init_blocks().await?;
                }

                // Try update again in any case
                Ok(true)
            }
            // Not found next block, but should exists on this step, return `true` for one more update
            None => Ok(true),
        }
    }
}

#[derive(Debug)]
pub struct Block {
    pub height: u32,
    pub hash: String,
    pub prevhash: Option<String>,
}

impl From<ResponseBlock> for Block {
    fn from(block: ResponseBlock) -> Self {
        Block {
            height: block.height,
            hash: block.hash.clone(),
            prevhash: block.previousblockhash,
        }
    }
}
