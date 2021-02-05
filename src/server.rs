use std::cell::RefCell;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, Mutex, Semaphore};
use tokio_util::codec::Framed;
use tracing::{debug, info};

use crate::context::WorkerManager;
use crate::handler::Handler;
use crate::message::codec::MQTT311;

pub(crate) type SyncWorkerManager = Mutex<WorkerManager>;

pub struct Server {
    listener: TcpListener,
    shutdown_rx: broadcast::Sender<()>,
    max_connections: Arc<Semaphore>,
    worker_manager: Arc<SyncWorkerManager>,
}

impl Server {
    pub async fn serve(&mut self) -> Result<TcpStream, Error> {
        info!("Telesteller server starts successfully.");

        loop {
            self.max_connections.acquire().await?.forget();

            let (mut socket, addr) = self.accept().await?;
            let mut transport = Framed::new(socket, MQTT311);
            let worker_manager = self.worker_manager.clone();
            let max_connections = self.max_connections.clone();
            tokio::spawn(async move {
                Handler::new(transport, addr, worker_manager, max_connections).serve().await;
            });
        }
    }

    async fn accept(&self) -> Result<(TcpStream, SocketAddr), Error> {
        let mut backoff = 1;

        loop {
            match self.listener.accept().await {
                Ok(result) => {
                    debug!(addr = ?&result.1, "TCP connection established.");
                    return Ok(result);
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
            worker_manager: Arc::new(Mutex::new(WorkerManager::new())),
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