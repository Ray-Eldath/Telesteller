use futures::SinkExt;
use tracing::{debug, warn};

use crate::handler::{Connection, Session, State};
use crate::message::{request::CONNECT, response};
use crate::message::codec::Transport;
#[macro_use]
use crate::require_state;

impl CONNECT {
    #[tracing::instrument(name = "CONNECT::apply", level = "debug", skip(transport))]
    pub(crate) async fn apply(self, conn: &mut Connection, transport: &mut Transport) -> Result<(), ()> {
        require_state!(CONNECT requires State::Established, conn);
        debug!("CONNECT received.");

        if self.protocol_version != 4 {
            transport.send(Box::new(response::CONNACK {
                session_present: false,
                return_code: response::CONNACKReturnCode::UnacceptableProtocol,
            })).await;
            return Err(());
        }

        // TODO: session management, ClientID / Username / Password verification
        transport.send(Box::new(response::CONNACK {
            session_present: false,
            return_code: response::CONNACKReturnCode::Accepted,
        })).await;

        conn.state = State::Connected(self, Session);

        Ok(())
    }
}