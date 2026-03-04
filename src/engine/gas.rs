use anyhow::Result;

use crate::engine::routing::Route;
use crate::types::ChainConfig;

#[derive(Debug, Clone)]
pub struct GasModel {
    pub max_gwei: f64,
    pub priority_gwei: f64,
    pub block_time: u64,
    pub usd_per_eth: f64,
}

impl GasModel {
    pub fn from_chain_config(chain: &ChainConfig, usd_per_eth: f64) -> Self {
        Self {
            max_gwei: chain.gas.max_gwei,
            priority_gwei: chain.gas.priority_gwei,
            block_time: chain.gas.block_time_seconds as u64,
            usd_per_eth,
        }
    }

    pub fn estimate_route_cost(&self, route: &Route) -> Result<f64> {
        let hops = route.pools.len() as u64;

        let base_gas: u64 = 220_000;
        let per_hop_gas: u64 = 140_000;
        let gas_units = base_gas + per_hop_gas.saturating_mul(hops.max(1));

        let gas_price_gwei = self.max_gwei.max(self.priority_gwei);
        let gas_price_eth = gas_price_gwei * 1e-9;

        let eth_cost = gas_units as f64 * gas_price_eth;
        Ok(eth_cost * self.usd_per_eth)
    }
}
