use std::time::{Duration, SystemTime};

use clap::ArgMatches;
use log::info;

mod app_state;
mod bitcoind;

use app_state::{AppState, Block};
use bitcoin_rust_learning::logger;
use bitcoind::RPCClient;

// Run server for monitoring bitcoin transactions
pub fn main(args: &ArgMatches) {
    logger::init();

    let state = AppState::default();
    let rpc = RPCClient::new(args.value_of("bitcoind").unwrap());

    let fut = sync_loop(state, rpc);
    actix_rt::System::new("sync_loop").block_on(fut);
}

// Bitcoind synchronize loop
async fn sync_loop(mut state: AppState, rpc: RPCClient) {
    loop {
        while state.blocks.len() < 6 {
            let hash = if state.blocks.is_empty() {
                rpc.getblockchaininfo().await.bestblockhash
            } else {
                state.blocks.front().unwrap().prevhash.clone()
            };

            match rpc.getblockheader(&hash).await {
                Some(block) => {
                    state.blocks.push_front(Block {
                        height: block.height,
                        hash: hash.clone(),
                        prevhash: block
                            .previousblockhash
                            .expect("Previous block hash should be defined"),
                    });
                    info!("Add block {}: {}", block.height, block.hash);
                }
                None => state.blocks.clear(),
            }
        }

        let ts = SystemTime::now();

        let last = state.blocks.back().unwrap();
        match rpc.getblockheader(&last.hash).await {
            Some(block) => {
                if let Some(hash) = block.nextblockhash {
                    match rpc.getblockheader(&hash).await {
                        Some(block) => {
                            state.blocks.push_back(Block {
                                height: block.height,
                                hash: block.hash.clone(),
                                prevhash: block
                                    .previousblockhash
                                    .expect("Previous block hash should be defined"),
                            });
                            state.blocks.pop_front();
                            info!("Add block {}: {}", block.height, block.hash);
                            continue;
                        }
                        None => continue,
                    }
                }
            }
            None => {
                state.blocks.pop_back();
                continue;
            }
        }

        // 25ms between calls, but minimum 5ms
        let elapsed = ts.elapsed().unwrap().as_micros() as u64;
        let time_to_sleep = std::cmp::max(25_1000 - elapsed, 5_1000);
        let sleep_duration = Duration::from_micros(time_to_sleep);
        actix_rt::time::delay_for(sleep_duration).await;
    }
}
