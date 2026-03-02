use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use super::events::{UniverseEvent, UniverseEvents};
use super::filter::UniverseFilter;
use super::types::Universe;

#[derive(Clone)]
pub struct UniverseManager {
    inner: Arc<RwLock<Universe>>,
    pub events: UniverseEvents,
}

impl UniverseManager {
    pub async fn new() -> Result<(Self, tokio::sync::broadcast::Receiver<UniverseEvent>)> {
        let filter = UniverseFilter::new();
        let uni = filter.build().await?;
        let (events, rx) = UniverseEvents::new();

        Ok((
            Self {
                inner: Arc::new(RwLock::new(uni)),
                events,
            },
            rx,
        ))
    }

    pub async fn get(&self) -> Universe {
        self.inner.read().await.clone()
    }

    pub async fn refresh(&self) -> Result<()> {
        let filter = UniverseFilter::new();
        let new_uni = filter.build().await?;

        {
            let mut guard = self.inner.write().await;
            *guard = new_uni;
        }

        let _ = self.events.tx.send(UniverseEvent::Updated);
        Ok(())
    }
}
