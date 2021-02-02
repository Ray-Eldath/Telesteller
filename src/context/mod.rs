use std::sync::Arc;

pub(crate) use worker::WorkerManager;

pub(crate) mod worker;

pub(crate) struct Context {
    worker_manager: Arc<WorkerManager>
}
