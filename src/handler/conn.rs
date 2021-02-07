use std::sync::Arc;

use futures::SinkExt;
use tracing::{debug, warn};

use crate::context::SessionManager;
use crate::handler::{Connection, Session, State};
use crate::message::{request::CONNECT, response};
use crate::message::codec::Transport;
#[macro_use]
use crate::require_state;
use crate::server::SyncSessionManager;

impl CONNECT {
    #[tracing::instrument(name = "CONNECT::apply", level = "debug", skip(transport, session_manager))]
    pub(crate) async fn apply(
        self,
        conn: &mut Connection,
        transport: &mut Transport,
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

        if self.clean_session {
            transport.send(Box::new(response::CONNACK {
                session_present: false,
                return_code: response::CONNACKReturnCode::Accepted,
            })).await;
            return Ok(());
        }

        let mut session = session_manager.write().await;
        let session = session.get(&self.client_id);
        let session_present = session.is_some();

        // TODO: ClientID / Username / Password verification
        transport.send(Box::new(response::CONNACK {
            session_present,
            return_code: response::CONNACKReturnCode::Accepted,
        })).await;

        conn.state = State::Connected(self,
                                      session.map(|s| s.clone())
                                          .unwrap_or(Session::default()));

        Ok(())
    }
}