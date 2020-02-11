use clap::ArgMatches;
use log::error;
use tokio::runtime;

use self::app::App;
use self::bitcoind::Bitcoind;
use self::error::AppError;
use super::logger;

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

    // create required structs
    let bitcoind = Bitcoind::new(bitcoind_url).map_err(AppError::Bitcoind)?;
    let app = App::new(bitcoind);

    // run app
    runtime::Builder::new()
        .basic_scheduler()
        .enable_io()
        .enable_time()
        .build()
        .expect("error on building runtime")
        .block_on(App::run(app))?;

    // TODO: add ^C handler, in such case return Ok(())
    Ok(())
}
