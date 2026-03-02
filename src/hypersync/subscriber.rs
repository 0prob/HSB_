use anyhow::Result;
use hypersync_client::{Client as HsClient};
use tokio_stream::StreamExt;
use tokio::time::{sleep, Duration};

use crate::engine::registry::PairRegistry;
use crate::types::ChainConfig;

use super::filters::build_filter;
use super::decode::decode_log;

/// Main HyperSync subscription loop for a single chain.
/// Dynamically updates filters as new pools are discovered.
pub async fn run_hypersync_subscriber(
    chain: ChainConfig,
    registry: PairRegistry,
) -> Result<()> {
    if !chain.enabled {
        tracing::info!("HyperSync disabled for chain {}", chain.name);
        return Ok(());
    }

    tracing::info!("Starting HyperSync subscriber for {}", chain.name);

    let client = HsClient::builder()
        .url(chain.hypersync_url.clone())
        .build()?;

    loop {
        let filter = build_filter(&chain, &registry);

        tracing::info!(
            "Subscribing to {} pools on {}",
            filter.addresses.as_ref().map(|a| a.len()).unwrap_or(0),
            chain.name
        );

        let mut stream = client.subscribe_logs(filter).await?;

        while let Some(log) = stream.next().await {
            let registry = registry.clone();
            let chain_name = chain.name.clone();

            tokio::spawn(async move {
                if let Err(e) = decode_log(chain_name, log, registry).await {
                    tracing::error!("Decode error: {:?}", e);
                }
            });
        }

        // If the stream ends, wait briefly and reconnect.
        sleep(Duration::from_secs(3)).await;
    }
}
