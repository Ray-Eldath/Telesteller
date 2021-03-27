use std::collections::HashSet;
use std::fmt::{self, Debug, Formatter};
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;
use tracing::warn;

use crate::message::codec::{DecodeError, MQTT311, Transport};
use crate::message::Qos;
use crate::message::request::{CONNECT, Request};
use crate::server::{SyncSessionManager, SyncWorkerManager};

mod conn;
mod pub_sub;
mod ping;

type Subscription = (String, Qos);

#[derive(Clone, Eq, Hash)]
pub(crate) struct DesignatedSubscription {
    topic: String,
    qos: Qos,
}

impl Debug for DesignatedSubscription {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {:?})", &self.topic, &self.qos)
    }
}

impl PartialEq for DesignatedSubscription {
    fn eq(&self, other: &Self) -> bool { self.topic == other.topic }
}

impl From<Subscription> for DesignatedSubscription {
    fn from(subscription: (String, Qos)) -> Self {
        DesignatedSubscription {
            topic: subscription.0,
            qos: subscription.1,
        }
    }
}

#[derive(PartialEq, Clone, Debug)]
pub(crate) struct Session {
    pub(crate) subscriptions: HashSet<DesignatedSubscription>
}

impl Default for Session {
    fn default() -> Self {
        Session {
            subscriptions: HashSet::default()
        }
    }
}

#[derive(Debug)]
enum State {
    /// TCP connection just established, and no handshake packages received.
    Established,
    /// CONNECT received and verified, CONNACK replied.
    Connected(CONNECT, Session),
    /// All Session data are persisted and current connection had been closed.
    Disconnected,
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
    worker_manager: Arc<SyncWorkerManager>,
    session_manager: Arc<SyncSessionManager>,
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
                            if let Err(_) = request.apply(&mut self.connection,
                                                          &mut self.transport,
                                                          &mut self.worker_manager,
                                                          &mut self.session_manager).await {
                                return; // Err indicates the Network Connection should be closed.
                            }
                        }
                        Request::SUBSCRIBE(request) => {
                            if let Err(_) = request.apply(&mut self.connection,
                                                          &mut self.transport,
                                                          &mut self.worker_manager,
                                                          &mut self.session_manager).await {
                                return;
                            }
                        }
                        Request::UNSUBSCRIBE(_) => {
                            // if there's subscriptions in the previous session, the connection has
                            // already intercepted by an infinite loop in CONNECT::apply, so code here
                            // only executed when a user trying to Unsubscribe without any subscriptions.
                            // this may causes a Protocol Violation, but here we just do nothing.
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
                        Request::DISCONNECT(request) => {
                            if let Err(_) = request.apply(&mut self.connection,
                                                          &mut self.transport,
                                                          &mut self.session_manager).await {
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
               session_manager: Arc<SyncSessionManager>,
               max_connections: Arc<Semaphore>) -> Handler {
        Handler {
            connection: Connection {
                addr,
                state: State::Established,
            },
            transport,
            worker_manager,
            session_manager,
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