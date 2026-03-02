use anyhow::Result;

use crate::engine::routing::Route;
use crate::types::ChainConfig;

/// Gas model for a chain.
#[derive(Clone)]
pub struct GasModel {
    pub max_gwei: f64,
    pub priority_gwei: f64,
    pub block_time: f64,
    pub usd_per_eth: f64,
}

impl GasModel {
    pub fn from_chain_config(cfg: &ChainConfig, usd_per_eth: f64) -> Self {
        Self {
            max_gwei: cfg.gas.max_gwei,
            priority_gwei: cfg.gas.priority_gwei,
            block_time: cfg.gas.block_time_seconds,
            usd_per_eth,
        }
    }

    /// Estimate gas cost for a route.
    /// This is intentionally simple — the ArbEngine will refine it.
    pub fn estimate_route_cost(&self, route: &Route) -> Result<f64> {
        let hops = route.pools.len() as f64;

        // Approximate gas per hop
        let gas_per_hop = 120_000.0;

        let total_gas = gas_per_hop * hops;

        // Convert gwei → ETH
        let gas_price_eth = (self.max_gwei / 1e9) * total_gas;

        // Convert ETH → USD
        let cost_usd = gas_price_eth * self.usd_per_eth;

        Ok(cost_usd)
    }
}
