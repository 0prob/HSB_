use anyhow::Result;
use ethers::types::Address;
use reqwest::Client;
use tokio::time::{sleep, Duration};

use crate::engine::registry::PairRegistry;
use crate::types::{PairMeta, ChainConfig};

use super::graphql::{AllPairs, all_pairs};

/// Poll HyperIndex for new pairs and update the registry.
/// This runs continuously in its own task.
pub async fn run_hyperindex_discovery(
    chain: ChainConfig,
    registry: PairRegistry,
    client: Client,
) -> Result<()> {
    if !chain.enabled {
        tracing::info!("HyperIndex discovery disabled for chain {}", chain.name);
        return Ok(());
    }

    tracing::info!("Starting HyperIndex discovery for {}", chain.name);

    loop {
        match fetch_pairs(&chain, &client).await {
            Ok(pairs) => {
                for p in pairs {
                    registry.insert(p);
                }
            }
            Err(e) => {
                tracing::error!("HyperIndex fetch error on {}: {:?}", chain.name, e);
            }
        }

        sleep(Duration::from_secs(10)).await;
    }
}

/// Fetch all PairCreated events from HyperIndex for a given chain.
async fn fetch_pairs(chain: &ChainConfig, client: &Client) -> Result<Vec<PairMeta>> {
    let vars = all_pairs::Variables {};
    let body = AllPairs::build_query(vars);

    let res = client
        .post(&chain.hyperindex_url)
        .json(&body)
        .send()
        .await?
        .json::<graphql_client::Response<all_pairs::ResponseData>>()
        .await?;

    let mut out = Vec::new();

    if let Some(data) = res.data {
        for p in data.pair_createds {
            let pool: Address = p.pair.parse()?;
            let token0: Address = p.token0.parse()?;
            let token1: Address = p.token1.parse()?;

            out.push(PairMeta {
                chain: chain.name.clone(),
                dex: p.dex.clone(),
                pool,
                token0,
                token1,
                fee_tier: None,
            });
        }
    }

    Ok(out)
}
