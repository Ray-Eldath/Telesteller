use std::pin::Pin;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use tokio::io::AsyncWriteExt;
use tokio::sync::{broadcast::error::RecvError};
use tokio_stream::{Stream, StreamMap};
use tracing::{debug, error, warn};

use crate::handler::{Connection, DesignatedSubscription, log_error, Subscription};
use crate::message::{Qos, Request, response};
use crate::message::codec::Transport;
use crate::message::request::{PUBLISH, SUBSCRIBE};
use crate::require_state;
use crate::server::{SyncSessionManager, SyncWorkerManager};

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
        match worker_manager.read().await.dispatch(&topic, self).await {
            Err(err) => {
                // TODO: Qos 1+ requires failure informing mechanism
                debug!(send_error = ?err, topic = &topic[..], "failed to dispatch.");
            }
            Ok(_) => { debug!("message dispatch successfully."); }
        }

        Ok(())
    }
}

type MessageStream = StreamMap<String, Pin<Box<dyn Stream<Item=Arc<PUBLISH>> + Send>>>;

impl SUBSCRIBE {
    #[tracing::instrument(name = "SUBSCRIBE::apply", level = "debug", skip(transport, worker_manager, session_manager))]
    pub(crate) async fn apply(
        self,
        conn: &mut Connection,
        transport: &mut Transport,
        worker_manager: &mut Arc<SyncWorkerManager>,
        session_manager: &mut Arc<SyncSessionManager>,
    ) -> Result<(), ()> {
        debug!("SUBSCRIBE received.");

        SUBSCRIBE::subscribe(&self.subscriptions, Some(self.id), conn, transport, worker_manager, session_manager).await?;
        transport.send(Box::new(response::SUBACK {
            id: self.id,
            granted_qos: self.subscriptions.iter().map(|_| Some(Qos::FireAndForget)).collect(),
        })).await;

        Ok(())
    }

    #[tracing::instrument(name = "SUBSCRIBE::subscribe", level = "debug", skip(transport, worker_manager, session_manager))]
    pub(crate) async fn subscribe(
        topics: &Vec<Subscription>,
        reply_to: Option<u16>,
        conn: &mut Connection,
        transport: &mut Transport,
        worker_manager: &mut Arc<SyncWorkerManager>,
        session_manager: &mut Arc<SyncSessionManager>,
    ) -> Result<(), ()> {
        require_state!(SUBSCRIBE requires State::Connected(..), &conn);

        let mut subscriptions = StreamMap::new();

        SUBSCRIBE::subscribe_topics(&topics, (transport, reply_to), conn, &mut subscriptions, worker_manager).await;

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
                                    SUBSCRIBE::subscribe_topics(&topics,
                                                                (transport, Some(request.id)),
                                                                conn,
                                                                &mut subscriptions,
                                                                worker_manager).await;
                                }
                                Request::UNSUBSCRIBE(request) => SUBSCRIBE::unsubscribe_topics(&request.topics, &mut subscriptions),
                                Request::PUBLISH(request) => request.apply(conn, transport, worker_manager).await?,
                                Request::PINGREQ(request) => request.apply(conn, transport).await?,
                                Request::DISCONNECT(request) => {
                                    request.apply(conn, transport, session_manager).await?;
                                    return Ok(());
                                },
                                _ => return Err(())
                            }
                        Err(err) => log_error(&conn.addr, err),
                    }
                }
            }
        }
    }

    #[tracing::instrument(name = "SUBSCRIBE::subscribe_topics", level = "debug", skip(reply_to, subscriptions, worker_manager))]
    async fn subscribe_topics(
        topics: &Vec<Subscription>,
        reply_to: (&mut Transport, Option<u16>),
        connection: &mut Connection,
        subscriptions: &mut MessageStream,
        worker_manager: &mut Arc<SyncWorkerManager>,
    ) {
        let session = match &mut connection.state {
            State::Connected(_, session) => session,
            _ => return,
        };

        let mut granted_qos = Vec::new();
        for (topic, qos) in topics {
            session.subscriptions.insert(DesignatedSubscription { topic: topic.clone(), qos: qos.clone() });

            let topic_handle = topic.clone();

            let mut subscriber = worker_manager.write().await.subscribe(&topic).await;

            let addr = connection.addr.clone();
            let topic = topic.clone();
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
            granted_qos.push(Some(Qos::FireAndForget));
        }

        if let (transport, Some(id)) = reply_to {
            transport.send(Box::new(response::SUBACK {
                id,
                granted_qos,
            })).await;
        }
    }

    fn unsubscribe_topics(topics: &Vec<String>, subscriptions: &mut MessageStream) {
        for topic in topics.iter() {
            subscriptions.remove(topic);
        }
    }
}