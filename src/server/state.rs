use std::collections::LinkedList;
use std::time::{Duration, SystemTime};

use log::info;

use super::bitcoind::{json::ResponseBlock, Bitcoind};
use super::error::{AppError, AppResult};
use crate::signals::ShutdownReceiver;

const APP_BLOCKS_MINIMUM: usize = 6;
const UPDATE_DELAY_MAX: Duration = Duration::from_millis(25);
const UPDATE_DELAY_MIN: Duration = Duration::from_millis(5);

#[derive(Debug)]
pub struct State {
    bitcoind: Bitcoind,
    blocks: LinkedList<Block>,
}

impl State {
    pub fn new(bitcoind: Bitcoind) -> State {
        State {
            bitcoind,
            blocks: LinkedList::new(),
        }
    }

    pub async fn run_update_loop(&mut self, mut shutdown: ShutdownReceiver) -> AppResult<()> {
        self.init_blocks(Some(&mut shutdown)).await?;

        loop {
            // Should we stop loop check
            if shutdown.is_recv() {
                break;
            }

            // Save current timestamp for timeout after check
            let ts = SystemTime::now();

            // Update our chain
            let blocks_modified = self.update_blocks().await?;
            if blocks_modified == UpdateBlocksModified::Yes {
                continue;
            }

            // Some delay if blocks chain was not modified
            let elapsed = ts.elapsed().unwrap();
            let sleep_duration = match UPDATE_DELAY_MAX.checked_sub(elapsed) {
                Some(delay) => std::cmp::max(delay, UPDATE_DELAY_MIN),
                None => UPDATE_DELAY_MIN,
            };

            // Exit earlier if shutdown signal received
            tokio::select! {
                _ = tokio::time::delay_for(sleep_duration) => {},
                _ = shutdown.recv() => { break },
            }
        }

        Ok(())
    }

    // Add block to our chain
    fn add_block(&mut self, block: Block, side: BlocksListSide) {
        let block = match side {
            BlocksListSide::Front => {
                self.remove_blocks(BlocksListSide::Back);
                self.blocks.push_front(block);
                self.blocks.front().unwrap()
            }
            BlocksListSide::Back => {
                self.remove_blocks(BlocksListSide::Front);
                self.blocks.push_back(block);
                self.blocks.back().unwrap()
            }
        };
        info!("Add block {}: {}", block.height, &block.hash);
    }

    fn remove_blocks(&mut self, side: BlocksListSide) {
        while self.blocks.len() >= APP_BLOCKS_MINIMUM {
            let block = match side {
                BlocksListSide::Front => self.blocks.pop_front().unwrap(),
                BlocksListSide::Back => self.blocks.pop_back().unwrap(),
            };
            info!("Remove block {}: {}", block.height, &block.hash);
        }
    }

    // Pop best block from our chain
    async fn remove_best_block(&mut self) -> AppResult<()> {
        self.blocks.pop_back();
        self.init_blocks(None).await
    }

    // Initialize our chain
    async fn init_blocks(&mut self, mut shutdown: Option<&mut ShutdownReceiver>) -> AppResult<()> {
        // Keep at least 6 blocks in chain
        while self.blocks.len() < APP_BLOCKS_MINIMUM {
            // Out from loop if we received shutdown signal
            if shutdown.is_some() && shutdown.as_mut().unwrap().is_recv() {
                break;
            }

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
            self.add_block(block, BlocksListSide::Front);
        }

        Ok(())
    }

    // Update our chain, return `true` if need call update again
    async fn update_blocks(&mut self) -> AppResult<UpdateBlocksModified> {
        // We always keep blocks, so unwrap is safe
        let mut last = self.blocks.back().unwrap();

        // Get bitcoind info
        let info_fut = self.bitcoind.getblockchaininfo();
        let info = info_fut.await.map_err(AppError::Bitcoind)?;

        // Best hash did not changed, return
        if info.bestblockhash == last.hash {
            return Ok(UpdateBlocksModified::No);
        }

        // Remove blocks in our chain on reorg
        while last.height >= info.blocks {
            self.remove_best_block().await?;
            last = self.blocks.back().unwrap();
        }

        // Add maximum 1 block
        let block_fut = self.bitcoind.getblockbyheight(last.height + 1);
        if let Some(block) = block_fut.await.map_err(AppError::Bitcoind)? {
            let block = Block::from(block);

            // If next block do not have previous blockhash, something wrong with blockchain
            if block.prevhash.is_none() {
                return Err(AppError::InvalidBlockchain);
            }

            // If previoush hash match to our best hash in new block, add it
            // Otherwise remove our best block
            if block.prevhash.as_ref().unwrap() == &last.hash {
                self.add_block(block, BlocksListSide::Back);
            } else {
                self.remove_best_block().await?;
            }
        }

        // Will force call `update_blocks` again immediately
        Ok(UpdateBlocksModified::Yes)
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

#[derive(Debug, PartialEq)]
enum BlocksListSide {
    Front,
    Back,
}

#[derive(Debug, PartialEq)]
enum UpdateBlocksModified {
    Yes,
    No,
}
