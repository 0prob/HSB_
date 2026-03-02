use anyhow::Result;
use dotenvy::dotenv;
use reqwest::Client;
use tokio::join;
use tokio::time::{sleep, Duration};
use tracing_subscriber::EnvFilter;

mod types;
mod engine {
    pub mod registry;
    pub mod normalize;
    pub mod pricing;
    pub mod routing;
    pub mod simulator;
    pub mod gas;
    pub mod arb;
}
mod hyperindex {
    pub mod graphql;
    pub mod discovery;
}
mod hypersync {
    pub mod subscriber;
    pub mod filters;
    pub mod decode;
}
mod dex {
    pub mod uniswap_v2;
    pub mod uniswap_v3;
    pub mod curve;
    pub mod balancer;
    pub mod algebra;
    pub mod quickswap;
    pub mod sushiswap;
}

use engine::{
    registry::PairRegistry,
    pricing::PricingEngine,
    gas::GasModel,
    arb::ArbEngine,
};
use types::ChainConfig;

/// Load a chain config from TOML.
fn load_chain(path: &str) -> Result<ChainConfig> {
    let raw = std::fs::read_to_string(path)?;
    let cfg: ChainConfig = toml::from_str(&raw)?;
    Ok(cfg)
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Load chain configs
    let polygon = load_chain("config/polygon.toml")?;
    let ethereum = load_chain("config/ethereum.toml")?;
    let arbitrum = load_chain("config/arbitrum.toml")?;
    let base = load_chain("config/base.toml")?;

    let chains = vec![polygon, ethereum, arbitrum, base];

    let registry = PairRegistry::new();
    let pricing = PricingEngine::new();
    let client = Client::new();

    // Spawn HyperIndex discovery tasks
    for chain in chains.clone() {
        let reg = registry.clone();
        let cli = client.clone();
        tokio::spawn(async move {
            let _ = hyperindex::discovery::run_hyperindex_discovery(
                chain,
                reg,
                cli,
            )
            .await;
        });
    }

    // Spawn HyperSync subscribers
    for chain in chains.clone() {
        let reg = registry.clone();
        tokio::spawn(async move {
            let _ = hypersync::subscriber::run_hypersync_subscriber(
                chain,
                reg,
            )
            .await;
        });
    }

    // Spawn ArbEngines
    for chain in chains.clone() {
        if !chain.enabled {
            continue;
        }

        let reg = registry.clone();
        let pr = pricing.clone();
        let gas = GasModel::from_chain_config(&chain, 3000.0); // TODO: dynamic ETH/USD

        let engine = ArbEngine::new(chain.clone(), reg, pr, gas);

        tokio::spawn(async move {
            loop {
                let _ = engine.act().await;
                sleep(Duration::from_millis(500)).await;
            }
        });
    }

    // Keep main alive
    loop {
        sleep(Duration::from_secs(60)).await;
    }
}
