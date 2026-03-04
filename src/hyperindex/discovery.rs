use anyhow::Result;
use ethers::types::Address;
use graphql_client::GraphQLQuery;
use reqwest::Client;
use tokio::time::{sleep, Duration};

use crate::engine::registry::PairRegistry;
use crate::types::{ChainConfig, PairMeta};
use crate::ui::UiHandle;

use super::graphql::{all_pairs, AllPairs};

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn chain_key(prefix: &str, chain: &str) -> String {
    format!("{}_{}", prefix, chain.to_uppercase())
}

/// Poll HyperIndex for PairCreated events and update the registry.
/// Reports progress to TUI as "loaded/target".
pub async fn run_hyperindex_discovery(
    chain: ChainConfig,
    registry: PairRegistry,
    client: Client,
    ui: Option<UiHandle>,
) -> Result<()> {
    if !chain.enabled {
        tracing::info!("HyperIndex discovery disabled for chain {}", chain.name);
        return Ok(());
    }

    let target = env_usize(&chain_key("HYPERINDEX_TARGET_POOLS", &chain.name), 0);

    if let Some(ui) = &ui {
        // if target not set, we’ll use UNIVERSE_MAX_POOLS_* (set by env) else fallback 5000
        let fallback_target = env_usize(&chain_key("UNIVERSE_MAX_POOLS", &chain.name), 5000);
        let t = if target == 0 { fallback_target } else { target };
        ui.init_chain(&chain.name, chain.enabled, t).await;
        ui.set_hyperindex_progress(&chain.name, 0, "starting").await;
    }

    tracing::info!("Starting HyperIndex discovery for {}", chain.name);

    loop {
        match fetch_pairs(&chain, &client).await {
            Ok(pairs) => {
                for p in pairs {
                    registry.insert(p);
                }

                let loaded = registry.by_chain(&chain.name).len();
                if let Some(ui) = &ui {
                    ui.set_hyperindex_progress(&chain.name, loaded, "loading").await;
                    // mark ready if >= target
                    let st = ui.state();
                    let t = {
                        let s = st.read().await;
                        s.chains.get(&chain.name).map(|c| c.hyperindex_target).unwrap_or(0)
                    };
                    if t > 0 && loaded >= t {
                        ui.set_hyperindex_progress(&chain.name, loaded, "ready").await;
                    }
                }
            }
            Err(e) => {
                tracing::error!("HyperIndex fetch error on {}: {:?}", chain.name, e);
                if let Some(ui) = &ui {
                    let loaded = registry.by_chain(&chain.name).len();
                    ui.set_hyperindex_progress(&chain.name, loaded, "error").await;
                }
            }
        }

        sleep(Duration::from_secs(10)).await;
    }
}

async fn fetch_pairs(chain: &ChainConfig, client: &Client) -> Result<Vec<PairMeta>> {
    let vars = all_pairs::Variables {};
    let body = AllPairs::build_query(vars);

    let res = client
        .post(&chain.hyperindex_url)
        .timeout(Duration::from_secs(10))
        .json(&body)
        .send()
        .await?
        .error_for_status()?
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
