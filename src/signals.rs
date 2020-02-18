use std::pin::Pin;

use futures::stream::{Stream, StreamExt as _};
use futures::task::{Context, Poll};
use log::{error, info};
use tokio::signal::unix;
use tokio::sync::broadcast;

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
        let mut finished: usize = 0;
        for idx in 0..self.streams.len() {
            match self.streams[idx].0.poll_recv(cx) {
                Poll::Pending => {}
                Poll::Ready(None) => {
                    finished += 1;
                    if finished == self.streams.len() {
                        return Poll::Ready(None);
                    }
                }
                Poll::Ready(Some(_)) => {
                    let sig = self.streams[idx].1;
                    return Poll::Ready(Some(sig));
                }
            }
        }
        Poll::Pending
    }
}

// TODO: Check tokio::sync and implement own version (education purpose only)
#[derive(Debug)]
pub struct ShutdownReceiver {
    tx: broadcast::Sender<()>,
    rx: broadcast::Receiver<()>,
    received: bool,
}

impl ShutdownReceiver {
    pub fn new() -> Self {
        let (tx, rx) = broadcast::channel::<()>(1);
        ShutdownReceiver {
            tx,
            rx,
            received: false,
        }
    }

    pub fn set(&mut self) {
        // unwrap is safe because `self` have Receiver for this Sender
        self.tx.send(()).unwrap();
    }

    pub fn is_recv(&mut self) -> bool {
        if !self.received {
            match self.rx.try_recv() {
                Ok(_) => {
                    self.received = true;
                }
                Err(broadcast::TryRecvError::Empty) => {}
                Err(err) => panic!("Shutdown channel error: {:?}", err),
            }
        }

        self.received
    }

    pub async fn recv(&mut self) {
        if !self.received {
            match self.rx.recv().await {
                Ok(_) => {
                    self.received = true;
                }
                Err(err) => panic!("Shutdown channel error: {:?}", err),
            }
        }
    }
}

impl Clone for ShutdownReceiver {
    fn clone(&self) -> Self {
        ShutdownReceiver {
            tx: self.tx.clone(),
            rx: self.tx.subscribe(),
            received: self.received,
        }
    }
}

pub fn subscribe() -> ShutdownReceiver {
    let shutdown = ShutdownReceiver::new();
    let mut notifier = shutdown.clone();

    tokio::spawn(async move {
        let mut s = Signals::new();

        if let Some(sig) = s.next().await {
            info!("{:?} received, shutting down...", sig);
            notifier.set();

            if let Some(sig) = s.next().await {
                info!("{:?} received, exit now...", sig);
            }
        }

        // In case if we received 2 signals, or tokio::signal return None
        std::process::exit(1);
    });

    shutdown
}
