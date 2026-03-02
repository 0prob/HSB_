use anyhow::Result;
use tokio::sync::Mutex;
use std::sync::Arc;
use ethers::types::U256;

use crate::engine::pricing::PricingEngine;
use crate::engine::routing::{RoutePlanner, Route, TriRoute};
use crate::engine::simulator::{ProfitSimulator, SimulationResult};
use crate::engine::gas::GasModel;
use crate::engine::registry::PairRegistry;
use crate::executor::builder::CalldataBuilder;
use crate::types::ChainConfig;

pub struct ArbEngine {
    pub chain: ChainConfig,
    pub routing: RoutePlanner,
    pub simulator: ProfitSimulator,
    pub last_opportunity: Arc<Mutex<Option<SimulationResult>>>,
}

impl ArbEngine {
    pub fn new(
        chain: ChainConfig,
        registry: PairRegistry,
        pricing: PricingEngine,
        gas_model: GasModel,
    ) -> Self {
        let routing = RoutePlanner::new(registry.clone(), pricing.clone());
        let simulator = ProfitSimulator::new(gas_model);

        Self {
            chain,
            routing,
            simulator,
            last_opportunity: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn evaluate_linear(&self) -> Result<Option<(Route, SimulationResult)>> {
        if !self.chain.enabled {
            return Ok(None);
        }

        let routes = self
            .routing
            .build_routes(&self.chain.name, self.chain.execution.max_route_hops);

        let mut best: Option<(Route, SimulationResult)> = None;

        for route in routes {
            let sim = self.simulator.simulate(
                &route,
                1.0,
                self.chain.execution.slippage_bps,
                self.chain.execution.min_profit_usd,
            )?;

            if sim.profitable {
                match &best {
                    Some((_, bsim)) if bsim.profit_usd >= sim.profit_usd => {}
                    _ => best = Some((route.clone(), sim.clone())),
                }
            }
        }

        Ok(best)
    }

    pub async fn evaluate_triangular(&self) -> Result<Option<(TriRoute, SimulationResult)>> {
        if !self.chain.enabled {
            return Ok(None);
        }

        let tris = self.routing.build_triangular_routes(&self.chain.name);
        let mut best: Option<(TriRoute, SimulationResult)> = None;

        for tri in tris {
            let sim = self.simulator.simulate_triangular(
                &tri,
                1.0,
                self.chain.execution.slippage_bps,
                self.chain.execution.min_profit_usd,
            )?;

            if sim.profitable {
                match &best {
                    Some((_, bsim)) if bsim.profit_usd >= sim.profit_usd => {}
                    _ => best = Some((tri.clone(), sim.clone())),
                }
            }
        }

        if let Some((_, ref sim)) = best {
            let mut guard = self.last_opportunity.lock().await;
            *guard = Some(sim.clone());
        }

        Ok(best)
    }

    /// Decide between linear (1–2 hop) and triangular (3 hop) and execute via bundle submission.
    pub async fn act(&self) -> Result<()> {
        let linear = self.evaluate_linear().await?;
        let tri = self.evaluate_triangular().await?;

        let mut best_kind: Option<&'static str> = None;
        let mut best_profit = f64::MIN;

        if let Some((_, sim)) = &linear {
            if sim.profit_usd > best_profit {
                best_profit = sim.profit_usd;
                best_kind = Some("linear");
            }
        }

        if let Some((_, sim)) = &tri {
            if sim.profit_usd > best_profit {
                best_profit = sim.profit_usd;
                best_kind = Some("triangular");
            }
        }

        let builder = CalldataBuilder;

        match best_kind {
            Some("linear") => {
                let (route, sim) = linear.unwrap();
                tracing::info!(
                    "[ARB] linear opportunity on {} profit=${:.4} pools={:?}",
                    self.chain.name,
                    sim.profit_usd,
                    route.pools
                );

                let encoded = builder.build_linear(
                    &route,
                    U256::from(1_000_000_000_000_000_000u128),
                    self.chain.execution.slippage_bps,
                    self.chain.executor.recipient,
                )?;

                crate::executor::bundle::submit_bundle(
                    &self.chain,
                    encoded,
                    sim.profit_usd,
                )
                .await?;
            }
            Some("triangular") => {
                let (tri, sim) = tri.unwrap();
                tracing::info!(
                    "[ARB] triangular opportunity on {} profit=${:.4} pools={:?}",
                    self.chain.name,
                    sim.profit_usd,
                    tri.pools
                );

                let encoded = builder.build_triangular(
                    &tri,
                    U256::from(1_000_000_000_000_000_000u128),
                    self.chain.execution.slippage_bps,
                    self.chain.executor.recipient,
                )?;

                crate::executor::bundle::submit_bundle(
                    &self.chain,
                    encoded,
                    sim.profit_usd,
                )
                .await?;
            }
            _ => {
                // no profitable opp
            }
        }

        Ok(())
    }
}
