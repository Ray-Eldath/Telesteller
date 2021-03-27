use std::sync::Arc;

use futures::SinkExt;
use tracing::{debug, warn};

use crate::handler::{Connection, Session, State};
use crate::message::{request::CONNECT, response};
use crate::message::codec::Transport;
use crate::message::request::{DISCONNECT, SUBSCRIBE};
use crate::require_state;
use crate::server::{SyncSessionManager, SyncWorkerManager};

impl CONNECT {
    #[tracing::instrument(name = "CONNECT::apply", level = "debug", skip(transport, worker_manager, session_manager))]
    pub(crate) async fn apply(
        self,
        conn: &mut Connection,
        transport: &mut Transport,
        worker_manager: &mut Arc<SyncWorkerManager>,
        session_manager: &mut Arc<SyncSessionManager>,
    ) -> Result<(), ()> {
        require_state!(CONNECT requires State::Established, conn);
        debug!("CONNECT received.");

        if self.protocol_version != 4 {
            transport.send(Box::new(response::CONNACK {
                session_present: false,
                return_code: response::CONNACKReturnCode::UnacceptableProtocol,
            })).await;
            return Err(());
        }

        let mut session = session_manager.write().await;
        if self.clean_session {
            session.evict(&self.client_id);
        }

        let session = session.get(&self.client_id);
        let session_present = session.is_some();
        let session = session.map(|s| s.clone())
            .unwrap_or(Session::default());
        let subscriptions = session.subscriptions.clone();

        debug!(addr = ?&conn.addr, session_present, session = ?&session, "Session retrieved or created");
        conn.state = State::Connected(self, session);

        // TODO: ClientID / Username / Password verification
        transport.send(Box::new(response::CONNACK {
            session_present,
            return_code: response::CONNACKReturnCode::Accepted,
        })).await;


        // if there's subscriptions in the previous session, restore them by
        // entering subscribed mode immediately.
        if session_present && !subscriptions.is_empty() {
            SUBSCRIBE::subscribe(&subscriptions.iter().map(|e| (e.topic.clone(), e.qos)).collect(),
                                 None,
                                 conn,
                                 transport,
                                 worker_manager,
                                 &mut session_manager.clone()).await;
        }

        Ok(())
    }
}

impl DISCONNECT {
    #[tracing::instrument(name = "DISCONNECT::apply", level = "debug", skip(transport, session_manager))]
    pub(crate) async fn apply(
        self,
        conn: &mut Connection,
        transport: &mut Transport,
        session_manager: &mut Arc<SyncSessionManager>,
    ) -> Result<(), ()> {
        require_state!(CONNECT requires State::Connected(..), conn);
        debug!("DISCONNECT received.");

        let (client_id, clean_session, session) = match &conn.state {
            State::Connected(CONNECT { client_id, clean_session, .. }, session) => (client_id, clean_session, session),
            _ => return Err(()),
        };
        if !clean_session {
            session_manager.write().await.put(&client_id, session.clone());
        }

        conn.state = State::Disconnected;
        transport.close().await;

        Ok(())
    }
}