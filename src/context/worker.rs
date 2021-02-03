use std::collections::HashMap;
use std::rc::Rc;

use tokio::sync::mpsc::{self, error::SendError};

use crate::message::Request;
use crate::util::ext::BoolExt;

pub(crate) struct WorkerManager {
    workers: Box<dyn WorkerRepository>
}

impl WorkerManager {
    async fn dispatch(&self, topic: &str, message: Request) -> Result<(), Vec<SendError<Rc<Request>>>> {
        let handle = Rc::new(message);

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

    fn new() -> WorkerManager {
        WorkerManager {
            workers: Box::new(DumbWorkerRepository::new())
        }
    }
}

type WorkerHandle = mpsc::Sender<Rc<Request>>;

trait WorkerRepository {
    fn find(&self, topic: &str) -> Vec<&WorkerHandle>;
    fn add(&mut self, topic: String, worker_handle: WorkerHandle);
    fn remove(&mut self, topic: &str);
    fn new() -> Self where Self: Sized;
}

struct DumbWorkerRepository {
    repository: HashMap<String, WorkerHandle>
}

impl WorkerRepository for DumbWorkerRepository {
    fn find(&self, topic: &str) -> Vec<&WorkerHandle> {
        self.repository.iter().filter_map(|(key, value)| {
            (key == topic).if_so(value)
        }).collect()
    }

    fn add(&mut self, topic: String, worker_handle: WorkerHandle) {
        self.repository.insert(topic, worker_handle);
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