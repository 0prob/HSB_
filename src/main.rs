use anyhow::Result;
use dotenvy::dotenv;
use reqwest::Client;
use tokio::time::{sleep, Duration};
use tracing_subscriber::EnvFilter;

mod types;
mod ui;
mod universe; // <-- REQUIRED so `crate::universe::...` resolves in the bin crate

mod engine {
    pub mod registry;
    pub mod normalize;
    pub mod pricing;
    pub mod routing;
    pub mod simulator;
    pub mod gas;
    pub mod arb;
    pub mod decimals;
    pub mod snapshot;
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
    pub mod algebra;
}

use engine::{
    arb::ArbEngine,
    decimals::DecimalsCache,
    gas::GasModel,
    pricing::PricingEngine,
    registry::PairRegistry,
    snapshot::RpcSnapshot,
};
use types::ChainConfig;

fn load_chain(path: &str) -> Result<ChainConfig> {
    let raw = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&raw)?)
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(default)
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let tui = env_bool("TUI", false);

    if tui {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("arbitrage.log")?;

        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_writer(file)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    }

    let ui_handle = if tui {
        let h = ui::UiHandle::new();
        h.set_status("starting…").await;
        Some(h)
    } else {
        None
    };

    // Load chain configs
    let polygon = load_chain("config/polygon.toml")?;
    let ethereum = load_chain("config/ethereum.toml")?;
    let arbitrum = load_chain("config/arbitrum.toml")?;
    let base = load_chain("config/base.toml")?;
    let chains = vec![polygon, ethereum, arbitrum, base];

    let registry = PairRegistry::new();
    let pricing = PricingEngine::new();
    let client = Client::new();

    // Start TUI task
    if let Some(h) = ui_handle.clone() {
        tokio::spawn(async move {
            let _ = ui::run_tui(h).await;
        });
    }

    if let Some(h) = &ui_handle {
        h.set_status("spawning discovery & streaming tasks…").await;
    }

    // HyperIndex discovery (pool catalog) with progress updates
    for chain in chains.clone() {
        let reg = registry.clone();
        let cli = client.clone();
        let ui = ui_handle.clone();
        tokio::spawn(async move {
            let _ = hyperindex::discovery::run_hyperindex_discovery(chain, reg, cli, ui).await;
        });
    }

    // HyperSync subscriber
    for chain in chains.clone() {
        let reg = registry.clone();
        let pr = pricing.clone();
        tokio::spawn(async move {
            let _ = hypersync::subscriber::run_hypersync_subscriber(chain, reg, pr).await;
        });
    }

    // Arb engines
    for chain in chains.clone() {
        if !chain.enabled {
            continue;
        }

        let reg = registry.clone();
        let pr = pricing.clone();

        let gas = GasModel::from_chain_config(&chain, 3000.0);
        let dec = DecimalsCache::new(&chain.rpc_url)?;
        let snap = RpcSnapshot::new(&chain.rpc_url)?;

        let engine = ArbEngine::new(chain.clone(), reg, pr, dec, snap, gas);

        let ui = ui_handle.clone();
        let chain_name = chain.name.clone();
        let reg_for_ui = registry.clone();

        tokio::spawn(async move {
            loop {
                if let Some(h) = &ui {
                    let pools = reg_for_ui.by_chain(&chain_name).len();
                    h.set_pools(&chain_name, pools).await;
                }
                let _ = engine.act().await;
                sleep(Duration::from_millis(500)).await;
            }
        });
    }

    loop {
        sleep(Duration::from_secs(60)).await;
    }
}
