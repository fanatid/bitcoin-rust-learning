use std::net::{SocketAddr, ToSocketAddrs as _};

use clap::ArgMatches;
use futures::stream::StreamExt as _;
use log::{error, info};
use tokio::sync::broadcast;

use self::api::run_server;
use self::bitcoind::Bitcoind;
use self::error::{AppError, AppResult};
use self::state::State;
use crate::logger;
use crate::signals::Signals;

mod api;
mod bitcoind;
mod error;
mod state;

// Initialize logging and execute run function
pub fn main(args: &ArgMatches) -> i32 {
    logger::init();

    // Create runtime and run app
    let app_result = tokio::runtime::Builder::new()
        .basic_scheduler()
        .enable_io()
        .enable_time()
        .build()
        .expect("error on building runtime")
        .block_on(run(args));

    if let Some(error) = app_result.err() {
        error!("{}", error);
        return 1;
    }

    0
}

// Run App for monitoring bitcoin blocks/transactions and HTTP/WS Server
// add explicit lifetime `'static` to the type of `args`: `&clap::args::arg_matches::ArgMatches<'static>`
#[allow(clippy::needless_lifetimes)]
async fn run<'a>(args: &ArgMatches<'a>) -> AppResult<()> {
    // Catch signals
    let (tx, rx1) = broadcast::channel::<()>(2);
    let rx2 = tx.subscribe();
    tokio::spawn(async move {
        let mut s = Signals::new();

        if let Some(sig) = s.next().await {
            info!("{:?} received, shutting down...", sig);
            tx.send(())
                .expect("Send shutdown notification successfully");
        }

        if let Some(sig) = s.next().await {
            info!("{:?} received, exit now...", sig);
            std::process::exit(2);
        }
    });

    // Create and validate bitcoind
    let bitcoind_url = args.value_of("bitcoind").unwrap();
    let mut bitcoind = Bitcoind::new(bitcoind_url).map_err(AppError::Bitcoind)?;
    bitcoind.validate().await.map_err(AppError::Bitcoind)?;

    // Create state
    let mut state = State::new(bitcoind);

    // Parse host:port
    let listen_arg = args.value_of("listen").unwrap();
    let listen_addr = listen_arg
        .to_socket_addrs()
        .map_err(AppError::ListenHostPortParse)?
        .find(|x| match x {
            SocketAddr::V4(_) => true,
            _ => false,
        })
        .ok_or(AppError::ListenHostPortNotFound)?;
    // Start HTTP/WS server
    run_server(listen_addr, rx2)?;

    // Run watch loop and block runtime
    state.run_update_loop(rx1).await
}
