use tokio::sync::mpsc::{self, error::SendError};
use regex::Regex;
use crate::context::Message;

pub(crate) struct WorkerManager {
    workers: Box<dyn WorkerRepository>
}

impl WorkerManager {
    async fn dispatch(&self, topic: &str, message: Message) -> Result<(), SendError<Message>> {
        if let Some(worker) = self.workers.find(topic) {
            return worker.send(message).await;
        }

        Ok(())
    }

    fn new() -> WorkerManager {
        WorkerManager {
            workers: Box::new(TrivialWorkerRepository::new())
        }
    }
}

type WorkerHandle = mpsc::Sender<Message>;

trait WorkerRepository {
    fn find(&self, topic: &str) -> Option<&WorkerHandle>;
    fn add(&mut self, topic: &str, worker_handle: WorkerHandle);
    fn remove(&mut self, topic: &str);
    fn new() -> Self where Self: Sized;
}

struct TrivialWorkerRepository {
    repository: Vec<(String, WorkerHandle)>
}

impl WorkerRepository for TrivialWorkerRepository {
    fn find(&self, topic: &str) -> Option<&WorkerHandle> {
        let regex =
            Regex::new(
                &(topic.replace("+", "([^\\/]+)")
                    .replace("/", "\\/") + "$")).unwrap();

        for (topic, handle) in self.repository.iter() {
            if regex.is_match(topic) {
                return Some(handle);
            }
        }

        None
    }

    fn add(&mut self, topic: &str, worker_handle: WorkerHandle) {
        &mut self.repository.push((topic.to_owned(), worker_handle));
    }

    fn remove(&mut self, topic: &str) {
        &mut self.repository.retain(|e| e.0 == topic);
    }

    fn new() -> TrivialWorkerRepository {
        TrivialWorkerRepository {
            repository: Vec::new()
        }
    }
}