use derive_more::Display;

#[derive(Debug, Display)]
pub enum AppError {
    #[display(fmt = "bitcoind: {}", _0)]
    Bitcoind(super::bitcoind::BitcoindError),

    #[display(fmt = "Listen host:port parse error: {}", _0)]
    ListenHostPortParse(std::io::Error),

    #[display(fmt = r#"Nothing to listen, please check "--listen" argument"#)]
    ListenHostPortNotFound,

    #[display(fmt = "Address ({}) bind error: {}", _0, _1)]
    HyperBind(std::net::SocketAddr, hyper::error::Error),

    #[display(fmt = "Not enough blocks for app")]
    NotEnoughBlocks,

    #[display(fmt = "Invalid blockchain")]
    InvalidBlockchain,
}

pub type AppResult<T> = Result<T, AppError>;
