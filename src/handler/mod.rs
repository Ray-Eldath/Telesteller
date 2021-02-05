use std::fmt::{self, Debug};
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::{Mutex, Semaphore};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::warn;

use crate::context::WorkerManager;
use crate::message::codec::{DecodeError, MQTT311, Transport};
use crate::message::request::{CONNECT, Request};
use crate::server::SyncWorkerManager;
use crate::util::Shutdown;

mod conn;
mod pub_sub;
mod ping;

#[derive(PartialEq)]
struct Session;

impl Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Session")
    }
}

#[derive(Debug, PartialEq)]
enum State {
    /// TCP connection just established, and no handshake packages received.
    Established,
    /// CONNECT received and verified, CONNACK replied.
    Connected(CONNECT, Session),
    /// Gracefully disconnect occurred, cleaning state.
    Disconnecting,
    /// Ungracefully disconnect occurred, cleaning state and sending LWT.
    Cleaning,
}

#[derive(Debug)]
pub(crate) struct Connection {
    state: State,
    addr: SocketAddr,
}

pub(crate) struct Handler {
    connection: Connection,
    transport: Transport,
    worker_manager: Arc<Mutex<WorkerManager>>,
    // shutdown: Shutdown,
    max_connections: Arc<Semaphore>,
}

#[macro_export]
macro_rules! require_state {
    ($type:ident requires $expected:pat, $s:expr) => {
        if let $expected = $s.state {}
        else {
            warn!(addr = ?$s.addr, frame = stringify!($type),
                    expected_state = stringify!($expected),
                    actual_state = ?$s.state,
                    "frame received when Handler of the connection is at invalid state.");
            return Err(());
        }
    };
}

#[allow(non_snake_case)]
impl Handler {
    pub async fn serve(&mut self) {
        while let Some(request) = self.transport.next().await {
            match request {
                Ok(request) =>
                    match request {
                        Request::CONNECT(request) => {
                            if let Err(_) = request.apply(&mut self.connection, &mut self.transport).await {
                                return; // Err indicates the Network Connection should be closed.
                            }
                        }
                        Request::SUBSCRIBE(request) => {
                            if let Err(_) = request.apply(&mut self.connection,
                                                          &mut self.transport,
                                                          &mut self.worker_manager).await {
                                return;
                            }
                        }
                        Request::PUBLISH(request) => {
                            if let Err(_) = request.apply(&self.connection,
                                                          &mut self.transport,
                                                          &mut self.worker_manager).await {
                                return;
                            }
                        }
                        Request::PINGREQ(request) => {
                            if let Err(_) = request.apply(&self.connection, &mut self.transport).await {
                                return;
                            }
                        }
                    }
                Err(err) => {
                    log_error(&self.connection.addr, err);
                    return;
                }
            }
        }
    }

    pub fn new(transport: Framed<TcpStream, MQTT311>,
               addr: SocketAddr,
               worker_manager: Arc<SyncWorkerManager>,
               max_connections: Arc<Semaphore>) -> Handler {
        Handler {
            connection: Connection {
                addr,
                state: State::Established,
            },
            transport,
            worker_manager,
            max_connections,
        }
    }
}

fn log_error(addr: &SocketAddr, err: DecodeError) {
    match err {
        DecodeError::Parsing(err) =>
            warn!(addr = ?addr, err = "DecodeError::Parsing", underlying_err = ?err),
        DecodeError::IO(err) =>
            warn!(addr = ?addr, err = "DecodeError::IO", underlying_err = ?err)
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        self.max_connections.add_permits(1);
    }
}