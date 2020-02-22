use clap::ArgMatches;
use futures::stream::StreamExt as _;
use log::{error, info};
use tokio_tungstenite::connect_async;
use url::Url;

use self::error::{AppError, AppResult};
use crate::logger;
use crate::signals;

mod error;

// Run WS client for transactions monitoring
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

#[allow(clippy::needless_lifetimes)]
async fn run<'a>(args: &ArgMatches<'a>) -> AppResult<()> {
    // Subscribe on shutdown signals
    let mut shutdown = signals::subscribe();

    let url = Url::parse(args.value_of("url").unwrap()).map_err(AppError::InvalidUrl)?;
    let (ws_stream, resp) = connect_async(url)
        .await
        .map_err(AppError::TungsteniteError)?;
    if resp.status().as_u16() != 101 {
        return Err(AppError::InvalidResponse(resp.status().as_u16()));
    }

    let (_, read) = ws_stream.split();
    let read_fut = read.for_each(|message| async {
        match message.unwrap().into_text() {
            Ok(text) => info!("{}", text),
            Err(err) => error!("{}", AppError::TungsteniteError(err)),
        };
    });

    tokio::select! {
        _ = shutdown.recv() => {},
        _ = read_fut => {},
    };

    Ok(())
}
