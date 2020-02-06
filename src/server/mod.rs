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
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Bitcoind(ref e) => write!(f, "bitcoind: {}", e),
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
        while state.blocks.len() < 6 {
            let hash = if state.blocks.is_empty() {
                rpc.getblockchaininfo()
                    .await
                    .map_err(ServerError::Bitcoind)?
                    .bestblockhash
            } else {
                state.blocks.front().unwrap().prevhash.clone()
            };

            match rpc.getblockheader(&hash).await {
                Ok(block) => {
                    state.blocks.push_front(Block {
                        height: block.height,
                        hash: hash.clone(),
                        prevhash: block
                            .previousblockhash
                            .expect("Previous block hash should be defined"),
                    });
                    info!("Add block {}: {}", block.height, block.hash);
                }
                Err(err) => match err {
                    RPCClientError::ResponseResultNotFound(_) => state.blocks.clear(),
                    _ => return Err(ServerError::Bitcoind(err)),
                },
            }
        }

        let ts = SystemTime::now();

        let last = state.blocks.back().unwrap();
        match rpc.getblockheader(&last.hash).await {
            Ok(block) => {
                if let Some(hash) = block.nextblockhash {
                    match rpc.getblockheader(&hash).await {
                        Ok(block) => {
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
                        Err(err) => match err {
                            RPCClientError::ResponseResultNotFound(_) => continue,
                            _ => return Err(ServerError::Bitcoind(err)),
                        },
                    }
                }
            }
            Err(err) => match err {
                RPCClientError::ResponseResultNotFound(_) => {
                    state.blocks.pop_back();
                    continue;
                }
                _ => return Err(ServerError::Bitcoind(err)),
            },
        }

        // 25ms between calls, but minimum 5ms
        let elapsed = ts.elapsed().unwrap().as_micros() as u64;
        let time_to_sleep = std::cmp::max(25_1000 - elapsed, 5_1000);
        let sleep_duration = Duration::from_micros(time_to_sleep);
        actix_rt::time::delay_for(sleep_duration).await;
    }
}
