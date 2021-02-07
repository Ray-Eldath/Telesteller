use lru::LruCache;

use crate::handler::Session;

pub(crate) struct SessionManager {
    sessions: Box<dyn SessionRepository + Sync + Send>
}

impl SessionManager {
    pub(crate) fn get(&mut self, client_id: &str) -> Option<&Session> { self.sessions.get(client_id) }
    pub(crate) fn get_mut(&mut self, client_id: &str) -> Option<&mut Session> { self.sessions.get_mut(client_id) }
    pub(crate) fn put(&mut self, client_id: &str, session: Session) { self.sessions.put(client_id, session) }

    pub(crate) fn new(size: usize) -> Self {
        SessionManager {
            sessions: Box::new(LruSessionRepository::new(size))
        }
    }
}

trait SessionRepository {
    fn get(&mut self, client_id: &str) -> Option<&Session>;
    fn get_mut(&mut self, client_id: &str) -> Option<&mut Session>;
    fn put(&mut self, client_id: &str, session: Session);
    fn new(size: usize) -> Self where Self: Sized;
}

struct LruSessionRepository {
    repository: LruCache<String, Session>
}

impl SessionRepository for LruSessionRepository {
    #[inline]
    fn get(&mut self, client_id: &str) -> Option<&Session> {
        self.repository.get(&client_id.to_owned())
    }

    #[inline]
    fn get_mut(&mut self, client_id: &str) -> Option<&mut Session> {
        self.repository.get_mut(&client_id.to_owned())
    }

    #[inline]
    fn put(&mut self, client_id: &str, session: Session) {
        self.repository.put(client_id.to_owned(), session);
    }

    fn new(size: usize) -> Self where Self: Sized {
        LruSessionRepository {
            repository: LruCache::new(size)
        }
    }
}