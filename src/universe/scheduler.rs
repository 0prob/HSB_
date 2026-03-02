use std::time::Duration;

use anyhow::Result;

use super::manager::UniverseManager;

pub struct UniverseScheduler {
    manager: UniverseManager,
    interval_hours: u64,
}

impl UniverseScheduler {
    pub fn new(manager: UniverseManager, interval_hours: u64) -> Self {
        Self {
            manager,
            interval_hours,
        }
    }

    pub async fn run(self) -> Result<()> {
        loop {
            tokio::time::sleep(Duration::from_secs(self.interval_hours * 3600)).await;

            if let Err(e) = self.manager.refresh().await {
                tracing::warn!("Universe refresh failed: {:?}", e);
            } else {
                tracing::info!("Universe refreshed and event emitted");
            }
        }
    }
}
