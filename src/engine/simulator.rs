use anyhow::Result;
use ethers::types::U256;

use crate::engine::routing::{Route, TriRoute};
use crate::engine::gas::GasModel;

/// Result of simulating a route.
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub expected_output: f64,
    pub gas_cost_usd: f64,
    pub profit_usd: f64,
    pub profitable: bool,
}

pub struct ProfitSimulator {
    gas: GasModel,
}

impl ProfitSimulator {
    pub fn new(gas: GasModel) -> Self {
        Self { gas }
    }

    /// Simulate a linear route.
    pub fn simulate(
        &self,
        route: &Route,
        input_amount: f64,
        slippage_bps: u32,
        min_profit_usd: f64,
    ) -> Result<SimulationResult> {
        let mut amount = input_amount;

        for _pool in &route.pools {
            amount *= route.price;
        }

        let slip = (slippage_bps as f64) / 10_000.0;
        amount *= 1.0 - slip;

        let gas_cost_usd = self.gas.estimate_route_cost(route)?;
        let profit = amount - input_amount - gas_cost_usd;

        Ok(SimulationResult {
            expected_output: amount,
            gas_cost_usd,
            profit_usd: profit,
            profitable: profit >= min_profit_usd,
        })
    }

    /// Simulate a triangular route A->B->C->A.
    pub fn simulate_triangular(
        &self,
        tri: &TriRoute,
        input_amount: f64,
        slippage_bps: u32,
        min_profit_usd: f64,
    ) -> Result<SimulationResult> {
        let mut amount = input_amount;

        // composite price is product of the 3 hops
        amount *= tri.composite_price;

        let slip = (slippage_bps as f64) / 10_000.0;
        amount *= 1.0 - slip;

        // approximate gas: 3 hops
        let fake_route = Route {
            pools: tri.pools.to_vec(),
            price: tri.composite_price,
        };
        let gas_cost_usd = self.gas.estimate_route_cost(&fake_route)?;
        let profit = amount - input_amount - gas_cost_usd;

        Ok(SimulationResult {
            expected_output: amount,
            gas_cost_usd,
            profit_usd: profit,
            profitable: profit >= min_profit_usd,
        })
    }
}
