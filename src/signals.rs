use std::pin::Pin;

use futures::stream::Stream;
use futures::task::{Context, Poll};

use log::error;
use tokio::signal::unix;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Signal {
    SIGINT,
    SIGTERM,
    SIGHUP,
    SIGQUIT,
}

#[derive(Debug)]
pub struct Signals {
    streams: Vec<(unix::Signal, Signal)>,
}

impl Signals {
    pub fn new() -> Signals {
        let sig_map = [
            (unix::SignalKind::interrupt(), Signal::SIGINT),
            (unix::SignalKind::terminate(), Signal::SIGTERM),
            (unix::SignalKind::hangup(), Signal::SIGHUP),
            (unix::SignalKind::quit(), Signal::SIGQUIT),
        ];

        let mut streams = Vec::with_capacity(sig_map.len());

        for (kind, sig) in sig_map.iter() {
            match unix::signal(*kind) {
                Ok(stream) => streams.push((stream, *sig)),
                Err(e) => error!("Can not initialize stream handler for {:?} err: {}", sig, e),
            }
        }

        Signals { streams }
    }
}

impl Stream for Signals {
    type Item = Signal;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        for idx in 0..self.streams.len() {
            match self.streams[idx].0.poll_recv(cx) {
                Poll::Pending => {}
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Ready(Some(_)) => {
                    let sig = self.streams[idx].1;
                    return Poll::Ready(Some(sig));
                }
            }
        }
        Poll::Pending
    }
}
