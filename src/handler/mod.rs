use std::sync::Arc;

use tokio::sync::Semaphore;

use crate::util::Shutdown;

struct Handler {
    max_connections: Arc<Semaphore>,
    shutdown: Shutdown,
}