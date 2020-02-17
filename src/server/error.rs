use std::io::Error as IOError;
use std::net::SocketAddr;

use hyper::error::Error as HyperError;

use super::bitcoind::BitcoindError;

quick_error! {
    #[derive(Debug)]
    pub enum AppError {
        Bitcoind(err: BitcoindError) {
            display("bitcoind: {}", err)
        }
        ListenHostPortParse(err: IOError) {
            display("Listen host:port parse error: {}", err)
        }
        ListenHostPortNotFound {
            display(r#"Nothing to listen, please check "--listen" argument"#)
        }
        HyperBind(addr: SocketAddr, err: HyperError) {
            display("Address ({}) bind error: {}", addr, err)
        }
        NotEnoughBlocks {
            display("Not enough blocks for app")
        }
        InvalidBlockchain {
            display("Invalid blockchain")
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
