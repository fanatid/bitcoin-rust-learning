use derive_more::Display;

#[derive(Debug, Display)]
pub enum BitcoindError {
    #[display(fmt = "Invalid URL ({})", _0)]
    InvalidUrl(url::ParseError),

    #[display(fmt = r#"URL scheme "{}" is not supported"#, _0)]
    InvalidUrlScheme(String),

    SendRequest(awc::error::SendRequestError),

    ResponsePayload(awc::error::PayloadError),

    #[display(fmt = "Invalid JSON response ({})", _0)]
    ResponseParse(serde_json::Error),

    #[display(fmt = "Nonce mismatch")]
    NonceMismatch,

    ResultRPC(super::json::ResponseError),

    #[display(fmt = "Bitcoind REST error (code: {}): {}", _0, _1)]
    ResultRest(u16, String),

    #[display(fmt = "Requested object not found")]
    ResultNotFound,

    #[display(fmt = "Result object not match to requested")]
    ResultMismatch,

    #[display(fmt = "Chain, height or best block hash did not match between clients")]
    ClientMismatch,
}
