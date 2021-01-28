use std::sync::Arc;

pub(crate) use message::Message;
pub(crate) use worker::WorkerManager;

pub(crate) mod worker;
pub(crate) mod message;

#[cfg(test)]
mod message_test;

pub(crate) struct Context {
    worker_manager: Arc<WorkerManager>
}
