use std::convert::Infallible;
use std::net::SocketAddr;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use log::info;

use super::error::{AppError, AppResult};
use crate::signals::ShutdownReceiver;

type ReqResult = Result<Response<Body>, Infallible>;

pub fn run_server(addr: SocketAddr, mut shutdown: ShutdownReceiver) -> AppResult<()> {
    let make_svc = make_service_fn(|_| async {
        let svc_fn = move |req| handle_request(req);

        Ok::<_, Infallible>(service_fn(svc_fn))
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

async fn handle_request(req: Request<Body>) -> ReqResult {
    let method = req.method();
    let path = req.uri().path();

    if method == Method::GET && path == "/mempool" {
        return get_mempool().await;
    }

    if method == Method::GET && path == "/block" {
        return get_block().await;
    }

    let resp = Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found"))
        .unwrap();

    Ok(resp)
}

async fn get_mempool() -> ReqResult {
    Ok(Response::new(Body::from("TODO")))
}

async fn get_block() -> ReqResult {
    Ok(Response::new(Body::from("TODO")))
}
