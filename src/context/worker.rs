use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::broadcast::{self, error::SendError, Sender};

use crate::message::request::PUBLISH;

pub(crate) struct WorkerManager {
    workers: Box<dyn WorkerRepository + Sync + Send>
}

impl WorkerManager {
    pub(crate) async fn dispatch(&self, topic: &str, message: PUBLISH) -> Result<(), Vec<SendError<Arc<PUBLISH>>>> {
        let handle = Arc::new(message);

        let mut errors = Vec::new();
        for worker in self.workers.find(topic).iter() {
            match worker.send(handle.clone()) {
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
        return match self.workers.find(topic) {
            Some(publisher) => publisher.subscribe(),
            None => {
                let (tx, rx) = broadcast::channel(1024);
                self.workers.add(topic, tx);
                rx
            }
        };
    }

    pub(crate) fn new() -> WorkerManager {
        WorkerManager {
            workers: Box::new(DumbWorkerRepository::new())
        }
    }
}

pub(crate) type Subscriber = broadcast::Receiver<Arc<PUBLISH>>;
pub(crate) type Publisher = broadcast::Sender<Arc<PUBLISH>>;

trait WorkerRepository {
    fn contains(&self, topic: &str) -> bool;
    fn find(&self, topic: &str) -> Option<&Publisher>;
    fn add(&mut self, topic: &str, publisher: Publisher);
    fn remove(&mut self, topic: &str);
    fn new() -> Self where Self: Sized;
}

struct DumbWorkerRepository {
    repository: HashMap<String, Publisher>
}

impl WorkerRepository for DumbWorkerRepository {
    fn contains(&self, topic: &str) -> bool { self.repository.contains_key(topic) }

    fn find(&self, topic: &str) -> Option<&Publisher> { self.repository.get(topic) }

    fn add(&mut self, topic: &str, publisher: Publisher) {
        self.repository.insert(topic.to_owned(), publisher);
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