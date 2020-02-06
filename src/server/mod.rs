use std::fmt;
use std::time::{Duration, SystemTime};

use clap::ArgMatches;
use log::{error, info};

mod app_state;
mod bitcoind;

use app_state::{AppState, Block};
use bitcoin_rust_learning::logger;
use bitcoind::rpc::{RPCClient, RPCClientError};

#[derive(Debug)]
enum ServerError {
    Bitcoind(RPCClientError),
    NotEnoughBlocks,
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ServerError::*;
        match *self {
            Bitcoind(ref e) => write!(f, "bitcoind: {}", e),
            NotEnoughBlocks => write!(f, "Not enough blocks for app"),
        }
    }
}

// Initialize logging and execute run function
pub fn main(args: &ArgMatches) -> i32 {
    logger::init();

    if let Some(error) = run(args).err() {
        error!("{}", error);
        return 1;
    }

    0
}

// Run server for monitoring bitcoin transactions
fn run(args: &ArgMatches) -> Result<(), ServerError> {
    // unwrap values from args, because existence should be validated by clap
    let bitcoind_url = args.value_of("bitcoind").unwrap();

    // create required structs
    let state = AppState::default();
    let rpc = RPCClient::new(bitcoind_url).map_err(ServerError::Bitcoind)?;

    // run loop
    let loop_sync = run_loop_sync(state, rpc);
    actix_rt::System::new("loop_sync").block_on(loop_sync)?;

    Ok(())
}

// Bitcoind synchronize loop
async fn run_loop_sync(mut state: AppState, rpc: RPCClient) -> Result<(), ServerError> {
    loop {
        // Keep at least 6 blocks in state
        while state.blocks_len() < 6 {
            // Get prevhash from first known block or just get tip
            let hash = if let Some(block) = state.blocks_first() {
                match block.prevhash {
                    None => return Err(ServerError::NotEnoughBlocks),
                    Some(ref hash) => hash.clone(),
                }
            } else {
                rpc.getblockchaininfo()
                    .await
                    .map_err(ServerError::Bitcoind)?
                    .bestblockhash
            };

            // Try fetch header
            let header = rpc
                .getblockheader(&hash)
                .await
                .map_err(ServerError::Bitcoind)?;

            // If header not found or error on adding block, reset state
            if let Some(header) = header {
                if let Ok(block) = state.blocks_push_front(Block::from(header)) {
                    info!("Add block {}: {}", block.height, &block.hash);
                    continue;
                };
            };

            state.reset();
        }

        // Save current timestamp for timeout after check
        let ts = SystemTime::now();

        // We always keep minimum 6 blocks, so unwrap is safe
        let last = state.blocks_last().unwrap();
        let header = rpc
            .getblockheader(&last.hash)
            .await
            .map_err(ServerError::Bitcoind)?;

        if let Some(header) = header {
            // Try fet next header
            if let Some(hash) = header.nextblockhash {
                let header = rpc
                    .getblockheader(&hash)
                    .await
                    .map_err(ServerError::Bitcoind)?;

                // All this wrong on edge cases
                if let Some(header) = header {
                    if let Ok(block) = state.blocks_push_back(Block::from(header)) {
                        info!("Add block {}: {}", block.height, &block.hash);
                        state.blocks_pop_first();
                        continue;
                    }
                }
            }
        } else {
            // Current header not found, drop it...
            state.blocks_pop_last();
            continue;
        }

        // 25ms between calls, but minimum 5ms
        let elapsed = ts.elapsed().unwrap().as_micros() as u64;
        let time_to_sleep = std::cmp::max(25_1000 - elapsed, 5_1000);
        let sleep_duration = Duration::from_micros(time_to_sleep);
        actix_rt::time::delay_for(sleep_duration).await;
    }
}
