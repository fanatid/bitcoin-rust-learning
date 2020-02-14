use clap::ArgMatches;
use futures::stream::StreamExt as _;
use log::{error, info};
use tokio::sync::oneshot;

use self::app::App;
use self::bitcoind::Bitcoind;
use self::error::AppError;
use crate::logger;
use crate::signals::Signals;

mod app;
mod bitcoind;
mod error;

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
fn run(args: &ArgMatches) -> Result<(), AppError> {
    // unwrap values from args, because existence should be validated by clap
    let bitcoind_url = args.value_of("bitcoind").unwrap();

    // Create required structs
    let bitcoind = Bitcoind::new(bitcoind_url).map_err(AppError::Bitcoind)?;
    let mut app = App::new(bitcoind);

    // Create runtime and run app
    tokio::runtime::Builder::new()
        .basic_scheduler()
        .enable_io()
        .enable_time()
        .build()
        .expect("error on building runtime")
        .block_on(async {
            let (tx, rx) = oneshot::channel();
            tokio::spawn(async {
                let mut s = Signals::new();

                if let Some(sig) = s.next().await {
                    info!("{:?} received, shutting down...", sig);
                    tx.send(()).unwrap();
                }

                if let Some(sig) = s.next().await {
                    info!("{:?} received, exit now...", sig);
                    std::process::exit(2);
                }
            });

            app.run(rx).await
        })
}
