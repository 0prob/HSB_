use anyhow::Result;
use ethers::types::{Bytes};
use reqwest::Client;
use serde::Serialize;

use crate::executor::types::EncodedRoute;
use crate::types::ChainConfig;

#[derive(Serialize)]
struct BundleTx {
    to: String,
    data: String,
}

#[derive(Serialize)]
struct BundleRequest {
    txs: Vec<BundleTx>,
    target_block: u64,
    metadata: BundleMetadata,
}

#[derive(Serialize)]
struct BundleMetadata {
    profit_usd: f64,
    chain: String,
}

/// Submit a bundle to a relay (Flashbots / bloXroute / Eden-style).
pub async fn submit_bundle(
    chain: &ChainConfig,
    encoded: EncodedRoute,
    profit_usd: f64,
) -> Result<()> {
    let relay_url = &chain.execution.bundle_relay_url;
    if relay_url.is_empty() {
        tracing::warn!("[BUNDLE] no relay configured for {}", chain.name);
        return Ok(());
    }

    let target_block = chain.execution.target_block_hint;

    let txs: Vec<BundleTx> = encoded
        .targets
        .iter()
        .zip(encoded.data.iter())
        .map(|(to, data)| BundleTx {
            to: format!("{:#x}", to),
            data: hex::encode(data),
        })
        .collect();

    let req = BundleRequest {
        txs,
        target_block,
        metadata: BundleMetadata {
            profit_usd,
            chain: chain.name.clone(),
        },
    };

    let client = Client::new();
    let mut builder = client.post(relay_url).json(&req);

    if let Some(h) = chain.execution.bundle_auth_header.as_ref() {
        builder = builder.header("Authorization", h);
    }

    let resp = builder.send().await?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        tracing::warn!(
            "[BUNDLE] relay error on {}: status={} body={}",
            chain.name,
            resp.status(),
            body
        );
    } else {
        tracing::info!(
            "[BUNDLE] submitted bundle on {} profit=${:.4}",
            chain.name,
            profit_usd
        );
    }

    Ok(())
}
