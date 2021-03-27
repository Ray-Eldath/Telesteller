use futures::SinkExt;
use tracing::{debug, warn};

use crate::handler::Connection;
use crate::message::codec::Transport;
use crate::message::request::PINGREQ;
use crate::message::response::PINGRESP;
use crate::require_state;

use super::State;

impl PINGREQ {
    #[tracing::instrument(name = "PINGREQ::apply", level = "debug", skip(transport))]
    pub(crate) async fn apply(self, conn: &Connection, transport: &mut Transport) -> Result<(), ()> {
        require_state!(PINGREQ requires State::Connected(..), &conn);
        debug!("PINGREQ received.");

        transport.send(Box::new(PINGRESP {})).await;

        Ok(())
    }
}