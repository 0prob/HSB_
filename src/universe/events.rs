use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum UniverseEvent {
    Updated,
}

#[derive(Clone)]
pub struct UniverseEvents {
    pub tx: broadcast::Sender<UniverseEvent>,
}

impl UniverseEvents {
    pub fn new() -> (Self, broadcast::Receiver<UniverseEvent>) {
        let (tx, rx) = broadcast::channel(16);
        (Self { tx }, rx)
    }
}
