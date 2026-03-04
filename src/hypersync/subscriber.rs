use anyhow::Result;
use tokio::time::{sleep, Duration};

use hypersync_client::{
    net_types::{LogField, Query},
    Client as HsClient,
    StreamConfig,
};

use crate::engine::pricing::PricingEngine;
use crate::engine::registry::PairRegistry;
use crate::types::ChainConfig;

use crate::universe::filter::UniverseFilter;

use super::decode::decode_log;
use super::filters::build_filter;

/// Stream logs for all pools currently in the registry.
/// Registry is primarily populated by HyperIndex; Polygon can optionally fallback to factory discovery (handled elsewhere).
pub async fn run_hypersync_subscriber(
    chain: ChainConfig,
    registry: PairRegistry,
    pricing: PricingEngine,
) -> Result<()> {
    if !chain.enabled {
        tracing::info!("HyperSync disabled for chain {}", chain.name);
        return Ok(());
    }

    tracing::info!("Starting HyperSync subscriber for {}", chain.name);

    let _universe = UniverseFilter::from_chain(&chain)?;

    let mut builder = HsClient::builder().chain_id(chain.chain_id);
    if let Ok(tok) = std::env::var("ENVIO_API_TOKEN") {
        builder = builder.api_token(tok);
    }
    let client = builder.build()?;

    let mut from_block: u64 = std::env::var("HYPERSYNC_FROM_BLOCK")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60_000_000);

    let window: u64 = std::env::var("HYPERSYNC_WINDOW_BLOCKS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50_000);

    loop {
        let pools = registry.by_chain(&chain.name);
        if pools.is_empty() {
            tracing::warn!("No pools in registry yet for {}; waiting...", chain.name);
            sleep(Duration::from_secs(2)).await;
            continue;
        }

        let filter = build_filter(&chain, &registry)?;

        tracing::info!(
            "Streaming swaps for {} pools on {} from_block={}",
            pools.len(),
            chain.name,
            from_block
        );

        let query = Query::new()
            .from_block(from_block)
            .to_block_excl(from_block.saturating_add(window))
            .where_logs(filter)
            .select_log_fields([
                LogField::Address,
                LogField::Topic0,
                LogField::Topic1,
                LogField::Topic2,
                LogField::Topic3,
                LogField::Data,
                LogField::TransactionHash,
                LogField::BlockNumber,
            ]);

        let mut rx = client.stream(query, StreamConfig::default()).await?;

        while let Some(msg) = rx.recv().await {
            let resp = msg?;
            from_block = resp.next_block;

            for batch in resp.data.logs {
                for lg in batch {
                    let reg = registry.clone();
                    let pr = pricing.clone();
                    let chain_name = chain.name.clone();
                    tokio::spawn(async move {
                        if let Err(e) = decode_log(chain_name, lg, reg, pr).await {
                            tracing::error!("Decode error: {:?}", e);
                        }
                    });
                }
            }
        }

        sleep(Duration::from_millis(250)).await;
    }
}
