use std::net::SocketAddr;
use std::time::Duration;
use std::sync::Arc;
use bytes::Bytes;
use structopt::StructOpt;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Semaphore, broadcast};
use tokio::io::AsyncReadExt;
use tracing::debug;
use thiserror::Error;
use crate::util::Shutdown;
use crate::context::Context;

pub struct Server {
    listener: TcpListener,
    shutdown_rx: broadcast::Sender<()>,
    max_connections: Arc<Semaphore>,
}

impl Server {
    pub async fn serve(&mut self) -> Result<TcpStream, Error> {
        loop {
            self.max_connections.acquire().await?.forget();

            let mut socket = self.accept().await?;

            let mut buffer = Vec::new();
            socket.read_to_end(&mut buffer).await?;
            Bytes::from(buffer);

            // let handler =
        }
    }

    async fn accept(&self) -> Result<TcpStream, Error> {
        loop {
            let mut backoff = 1;

            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    debug!(addr = ?addr, "TCP connection established.");
                    return Ok(stream);
                }
                Err(err) => {
                    if backoff > 60 {
                        return Err(Error::AcceptError(err));
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(backoff)).await;

            backoff *= 2;
        }
    }

    pub fn new(listener: TcpListener, shutdown_rx: broadcast::Sender<()>, max_connections: Arc<Semaphore>) -> Server {
        Server {
            listener,
            shutdown_rx,
            max_connections,
        }
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot accept new connection due to resource limitation: {0:?}")]
    AcquireError(#[from] tokio::sync::AcquireError),
    #[error("failed to accept connection: {0:?}")]
    AcceptError(std::io::Error),
    #[error("cannot read the TcpStream: {0:?}")]
    ReadError(#[from] std::io::Error),
}