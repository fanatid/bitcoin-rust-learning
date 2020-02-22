use tokio_tungstenite::tungstenite::error::Error as TungsteniteError;
use url::ParseError as UrlParseError;

quick_error! {
    #[derive(Debug)]
    pub enum AppError {
        InvalidUrl(err: UrlParseError) {
            display("Invalid URL ({})", err)
        }
        TungsteniteError(err: TungsteniteError) {
            display("WebSocket error: {}", err)
        }
        InvalidResponse(status: u16) {
            display("Invalid response statuc: {}", status)
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
