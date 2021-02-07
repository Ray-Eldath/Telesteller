use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::broadcast::{self, error::SendError};

use crate::message::request::PUBLISH;

pub(crate) struct PublisherManager {
    publishers: Box<dyn PublisherRepository + Sync + Send>
}

impl PublisherManager {
    pub(crate) async fn dispatch(&self, topic: &str, message: PUBLISH) -> Result<(), SendError<Arc<PUBLISH>>> {
        let handle = Arc::new(message);
        if let Some(publisher) = self.publishers.find(topic) {
            publisher.send(handle)?;
        }

        Ok(())
    }

    pub(crate) async fn subscribe(&mut self, topic: &str) -> Subscriber {
        return match self.publishers.find(topic) {
            Some(publisher) => publisher.subscribe(),
            None => {
                let (tx, rx) = broadcast::channel(1024);
                self.publishers.add(topic, tx);
                rx
            }
        };
    }

    pub(crate) fn new() -> PublisherManager {
        PublisherManager {
            publishers: Box::new(DumbPublisherRepository::new())
        }
    }
}

pub(crate) type Subscriber = broadcast::Receiver<Arc<PUBLISH>>;
pub(crate) type Publisher = broadcast::Sender<Arc<PUBLISH>>;

trait PublisherRepository {
    fn contains(&self, topic: &str) -> bool;
    fn find(&self, topic: &str) -> Option<&Publisher>;
    fn add(&mut self, topic: &str, publisher: Publisher);
    fn remove(&mut self, topic: &str);
    fn new() -> Self where Self: Sized;
}

struct DumbPublisherRepository {
    repository: HashMap<String, Publisher>
}

impl PublisherRepository for DumbPublisherRepository {
    fn contains(&self, topic: &str) -> bool { self.repository.contains_key(topic) }

    fn find(&self, topic: &str) -> Option<&Publisher> { self.repository.get(topic) }

    fn add(&mut self, topic: &str, publisher: Publisher) {
        self.repository.insert(topic.to_owned(), publisher);
    }

    fn remove(&mut self, topic: &str) {
        self.repository.retain(|key, _| key != topic)
    }

    fn new() -> Self where Self: Sized {
        DumbPublisherRepository {
            repository: HashMap::new()
        }
    }
}