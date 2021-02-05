use std::pin::Pin;
use std::sync::Arc;

use bytes::BufMut;
use futures::StreamExt;
use tokio::io::AsyncWriteExt;
use tokio::sync::broadcast::error::RecvError;
use tokio_stream::{Stream, StreamMap};
use tracing::{debug, error, warn};
use tracing_subscriber::fmt::format::debug_fn;

use crate::handler::{Connection, log_error};
use crate::message::{Request, request::PUBLISH};
use crate::message::codec::Transport;
use crate::message::request::Request::PINGREQ;
use crate::message::request::SUBSCRIBE;
#[macro_use]
use crate::require_state;
use crate::server::SyncWorkerManager;

use super::State;

impl PUBLISH {
    #[tracing::instrument(name = "PUBLISH::apply", level = "debug", skip(transport, worker_manager))]
    pub(crate) async fn apply(
        self,
        conn: &Connection,
        transport: &mut Transport,
        worker_manager: &Arc<SyncWorkerManager>,
    ) -> Result<(), ()> {
        require_state!(PUBLISH requires State::Connected(..), &conn);
        debug!("PUBLISH received.");

        let topic = self.topic.clone();
        match worker_manager.lock().await.dispatch(&topic, self).await {
            Err(err) => {
                // TODO: Qos 1+ requires failure informing mechanism
                debug!("failed to dispatch.")
            }
            Ok(_) => { debug!("message dispatch successfully."); }
        }

        Ok(())
    }
}

impl SUBSCRIBE {
    #[tracing::instrument(name = "SUBSCRIBE::apply", level = "debug", skip(transport, worker_manager))]
    pub(crate) async fn apply(
        self,
        conn: &mut Connection,
        transport: &mut Transport,
        worker_manager: &mut Arc<SyncWorkerManager>,
    ) -> Result<(), ()> {
        require_state!(SUBSCRIBE requires State::Connected(..), &conn);
        debug!("SUBSCRIBE received.");

        let mut subscriptions = StreamMap::new();

        SUBSCRIBE::subscribe_request(self, conn, &mut subscriptions, transport, worker_manager).await;

        loop {
            tokio::select! {
                Some((topic, message)) = subscriptions.next() => {
                    debug!(topic = &topic[..], "received message from subscription");
                    transport.get_mut().write(&message.raw).await;
                }
                Some(request) = transport.next() => {
                    match request {
                        Ok(request) =>
                            match request {
                                Request::SUBSCRIBE(request) => {
                                    SUBSCRIBE::subscribe_request(request, conn, &mut subscriptions, transport, worker_manager).await;
                                }
                                Request::PUBLISH(request) => {
                                    if let Err(_) = request.apply(conn, transport, worker_manager).await {
                                        return Err(());
                                    }
                                }
                                Request::PINGREQ(request) => {
                                    if let Err(_) = request.apply(conn, transport).await {
                                        return Err(());
                                    }
                                }
                                _ => return Err(())
                            }
                        Err(err) => log_error(&conn.addr, err),
                    }
                }
            }
        }
    }

    async fn subscribe_request(
        request: SUBSCRIBE,
        connection: &Connection,
        subscriptions: &mut StreamMap<String, Pin<Box<dyn Stream<Item=Arc<PUBLISH>> + Send>>>,
        transport: &Transport,
        worker_manager: &mut Arc<SyncWorkerManager>,
    ) {
        for (topic, qos) in request.subscriptions {
            let topic_handle = topic.clone();

            let mut subscriber = worker_manager.lock().await.subscribe(&topic).await;

            let addr = connection.addr.clone();
            let stream = Box::pin(async_stream::stream! {
                loop {
                    match subscriber.recv().await {
                        Ok(msg) => yield msg,
                        // Lagged happened when the channel is full, we print a log and do nothing.
                        Err(RecvError::Lagged(_)) =>
                            error!(addr = ?addr, topic = &topic[..],
                                    "broadcast channel of the topic is full, newly incoming messages publishing to this topic will be discarded directly."),
                        Err(_) => break,
                    }
                }
            });

            subscriptions.insert(topic_handle, stream);
        }
    }
}