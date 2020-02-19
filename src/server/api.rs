use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use log::info;
use regex::{Captures, Regex};

use super::error::{AppError, AppResult};
use super::state::State;
use crate::signals::ShutdownReceiver;

type ReqResult = Result<Response<Body>, Infallible>;

pub fn run_server(
    addr: SocketAddr,
    state: Arc<State>,
    mut shutdown: ShutdownReceiver,
) -> AppResult<()> {
    let make_svc = make_service_fn(move |_| {
        let state = state.clone();
        async move { Ok::<_, Infallible>(service_fn(move |req| handle_request(state.clone(), req))) }
    });

    let server = Server::try_bind(&addr)
        .map_err(|err| AppError::HyperBind(addr, err))?
        .http1_only(true)
        .tcp_nodelay(true)
        .serve(make_svc);

    let local_addr = server.local_addr();
    info!("Start API server at {}", local_addr);

    // TODO: Check hyper::Server, becuase I do not understand:
    // Why it's ok for `server`, but for `shutdown`: borrowed value does not live long enough
    tokio::spawn(server.with_graceful_shutdown(async move { shutdown.recv().await }));

    Ok(())
}

// TODO: implement router (education?)
async fn handle_request(state: Arc<State>, req: Request<Body>) -> ReqResult {
    let method = req.method();
    let path = req.uri().path().to_string();

    if method == Method::GET && path == "/mempool" {
        return get_mempool().await;
    }

    let re = Regex::new(r"^/block/([0-9a-f]{4}|\d+|tip)$").unwrap();
    let caps = re.captures(&path);
    if method == Method::GET && caps.is_some() {
        return get_block(state, req, caps.unwrap()).await;
    }

    let resp = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found"))
        .unwrap();

    Ok(resp)
}

// fn handle_request_on_error(err: Box<dyn fmt::Display>) -> ReqResult {
//     let body = format!("{}", err);
//     Ok(Response::builder()
//         .status(StatusCode::INTERNAL_SERVER_ERROR)
//         .body(Body::from(body))
//         .unwrap())
// }

async fn get_mempool() -> ReqResult {
    Ok(Response::new(Body::from("TODO")))
}

async fn get_block<'t>(state: Arc<State>, req: Request<Body>, caps: Captures<'t>) -> ReqResult {
    let id = caps.get(1).unwrap().as_str();
    let block = if id == "tip" {
        state.get_block_tip().await
    } else if id.len() == 64 {
        state.get_block_by_hash(id).await
    } else {
        let height = id.parse::<u32>().unwrap();
        state.get_block_by_height(height).await
    };

    let data = serde_json::to_string(&block.unwrap().unwrap()).unwrap();
    Ok(Response::new(Body::from(data)))
}
