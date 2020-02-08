use derive_more::Display;

#[derive(Debug, Display)]
pub enum AppError {
    #[display(fmt = "bitcoind: {}", _0)]
    Bitcoind(super::bitcoind::BitcoindError),

    #[display(fmt = "Not enough blocks for app")]
    NotEnoughBlocks,

    #[display(fmt = "Invalid blockchain")]
    InvalidBlockchain,
}
