use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::mpsc::{self, error::SendError};

use crate::message::request::PUBLISH;

pub(crate) struct WorkerManager {
    workers: Box<dyn WorkerRepository + Sync + Send>
}

impl WorkerManager {
    pub(crate) async fn dispatch(&self, topic: &str, message: PUBLISH) -> Result<(), Vec<SendError<Arc<PUBLISH>>>> {
        let handle = Arc::new(message);

        let mut errors = Vec::new();
        for worker in self.workers.find(topic).iter() {
            match worker.send(handle.clone()).await {
                Err(e) => errors.push(e),
                _ => {}
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub(crate) async fn subscribe(&mut self, topic: &str) -> Subscriber {
        let (sender, receiver) = mpsc::channel(1024);
        self.workers.add(topic, sender);
        receiver
    }

    pub(crate) fn new() -> WorkerManager {
        WorkerManager {
            workers: Box::new(DumbWorkerRepository::new())
        }
    }
}

pub(crate) type Subscriber = mpsc::Receiver<Arc<PUBLISH>>;
pub(crate) type WorkerHandle = mpsc::Sender<Arc<PUBLISH>>;

trait WorkerRepository {
    fn find(&self, topic: &str) -> Vec<&WorkerHandle>;
    fn add(&mut self, topic: &str, worker_handle: WorkerHandle);
    fn remove(&mut self, topic: &str);
    fn new() -> Self where Self: Sized;
}

struct DumbWorkerRepository {
    repository: HashMap<String, Vec<Box<WorkerHandle>>>
}

impl WorkerRepository for DumbWorkerRepository {
    fn find(&self, topic: &str) -> Vec<&WorkerHandle> {
        match self.repository.get(topic) {
            Some(handles) =>
                handles.iter().map(|e| &**e).collect(),
            None => Vec::new()
        }
    }

    fn add(&mut self, topic: &str, worker_handle: WorkerHandle) {
        match self.repository.entry(topic.to_owned()) {
            Entry::Occupied(mut entry) =>
                entry.get_mut().push(Box::new(worker_handle)),
            Entry::Vacant(entry) =>
                { entry.insert(Vec::new()); }
        };
    }

    fn remove(&mut self, topic: &str) {
        self.repository.retain(|key, _| key != topic)
    }

    fn new() -> Self where Self: Sized {
        DumbWorkerRepository {
            repository: HashMap::new()
        }
    }
}