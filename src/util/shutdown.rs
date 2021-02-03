use tokio::sync::broadcast;

pub(crate) struct Shutdown {
    shutdown: bool,
    signal: broadcast::Receiver<()>,
}

impl Shutdown {
    pub(crate) fn new(recv: broadcast::Receiver<()>) -> Shutdown {
        Shutdown {
            shutdown: false,
            signal: recv,
        }
    }

    pub(crate) fn is_shutdown(&self) -> bool { self.shutdown }

    pub(crate) async fn poll(&mut self) {
        if self.shutdown {
            return;
        }

        let _ = self.signal.recv().await;
        self.shutdown = true;
    }
}