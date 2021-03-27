use std::collections::HashMap;

use crate::handler::Session;

pub(crate) struct SessionManager {
    sessions: Box<dyn SessionRepository + Sync + Send>
}

impl SessionManager {
    pub(crate) fn get(&self, client_id: &str) -> Option<&Session> { self.sessions.get(client_id) }
    pub(crate) fn get_mut(&mut self, client_id: &str) -> Option<&mut Session> { self.sessions.get_mut(client_id) }
    pub(crate) fn put(&mut self, client_id: &str, session: Session) { self.sessions.put(client_id, session) }
    pub(crate) fn evict(&mut self, client_id: &str) { self.sessions.evict(client_id) }

    pub(crate) fn new(size: usize) -> Self {
        SessionManager {
            sessions: Box::new(HashMapSessionRepository::new())
        }
    }
}

trait SessionRepository {
    fn get(&self, client_id: &str) -> Option<&Session>;
    fn get_mut(&mut self, client_id: &str) -> Option<&mut Session>;
    fn put(&mut self, client_id: &str, session: Session);
    fn evict(&mut self, client_id: &str);
    fn new() -> Self where Self: Sized;
}

struct HashMapSessionRepository {
    repository: HashMap<String, Session>
}

impl SessionRepository for HashMapSessionRepository {
    fn get(&self, client_id: &str) -> Option<&Session> {
        self.repository.get(&client_id.to_owned())
    }

    fn get_mut(&mut self, client_id: &str) -> Option<&mut Session> {
        self.repository.get_mut(&client_id.to_owned())
    }

    fn put(&mut self, client_id: &str, session: Session) {
        for (client_id, session) in self.repository.iter() {
            println!("({}, {:?}) ", client_id, session);
        }

        self.repository.insert(client_id.to_owned(), session);
    }

    fn evict(&mut self, client_id: &str) {
        self.repository.remove(client_id);
    }

    fn new() -> Self where Self: Sized {
        HashMapSessionRepository {
            repository: HashMap::new()
        }
    }
}