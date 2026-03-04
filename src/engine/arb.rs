use anyhow::Result;
use ethers::types::{Address, U256};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};

use crate::engine::decimals::DecimalsCache;
use crate::engine::gas::GasModel;
use crate::engine::pricing::PricingEngine;
use crate::engine::registry::PairRegistry;
use crate::engine::routing::{RoutePlanner, Route, TriRoute};
use crate::engine::simulator::{ProfitSimulator, SimulationResult};
use crate::engine::snapshot::RpcSnapshot;
use crate::types::ChainConfig;

use crate::universe::filter::UniverseFilter;

pub struct ArbEngine {
    pub chain: ChainConfig,
    pub routing: RoutePlanner,
    pub simulator: ProfitSimulator,
    pub decimals: DecimalsCache,
    pub snapshot: RpcSnapshot,
    pub universe: UniverseFilter,
}

impl ArbEngine {
    pub fn new(
        chain: ChainConfig,
        registry: PairRegistry,
        pricing: PricingEngine,
        decimals: DecimalsCache,
        snapshot: RpcSnapshot,
        gas_model: GasModel,
    ) -> Self {
        let routing = RoutePlanner::new(registry.clone(), pricing.clone());
        let simulator = ProfitSimulator::new(gas_model, registry, pricing, decimals.clone());
        let universe = UniverseFilter::from_chain(&chain).expect("UniverseFilter init failed");
        Self { chain, routing, simulator, decimals, snapshot, universe }
    }

    fn usdc_addr() -> Address {
        "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse().unwrap()
    }
    fn usdt_addr() -> Address {
        "0xC2132D05D31c914a87C6611C10748AEb04B58e8F".parse().unwrap()
    }
    fn dai_addr() -> Address {
        "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063".parse().unwrap()
    }

    fn env_usize(key: &str, default: usize) -> usize {
        std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
    }

    async fn notional_in_token(&self, token: Address, dollars: f64) -> U256 {
        let dec = self.decimals.get_or_fetch(token).await.unwrap_or(18);
        let scale = 10f64.powi(dec as i32);
        let v = (dollars * scale).round();
        if v <= 0.0 { return U256::zero(); }
        U256::from(v as u128)
    }

    async fn evaluate_best_2hop(&self, snap_block: u64) -> Result<Option<(Route, SimulationResult)>> {
        let stables = [Self::usdc_addr(), Self::usdt_addr(), Self::dai_addr()];
        let dollars = 100.0;

        self.decimals.preload(stables).await;

        let mut routes = self.routing.build_2hop_cycles(&self.chain.name);
        routes.retain(|r| self.universe.accept_route_tokens(&r.tokens));
        routes.retain(|r| r.tokens.len() == 3 && r.tokens[0] == r.tokens[2] && stables.contains(&r.tokens[0]));
        routes.sort_by(|a,b| b.price.partial_cmp(&a.price).unwrap_or(std::cmp::Ordering::Equal));

        let top_k = Self::env_usize("SNAPSHOT_TOPK", 25).min(routes.len());
        let routes = routes.into_iter().take(top_k).collect::<Vec<_>>();
        if routes.is_empty() { return Ok(None); }

        let conc = Self::env_usize("SNAPSHOT_CONCURRENCY", 8).max(1);
        let sem = Arc::new(Semaphore::new(conc));
        let best: Arc<Mutex<Option<(Route, SimulationResult)>>> = Arc::new(Mutex::new(None));
        let mut handles = Vec::with_capacity(routes.len());

        for route in routes {
            let permit = sem.clone().acquire_owned().await?;
            let best = best.clone();
            let snapshot = self.snapshot.clone();
            let simulator = self.simulator.clone();
            let slippage_bps = self.chain.execution.slippage_bps;
            let min_profit_usd = self.chain.execution.min_profit_usd;

            let start = route.tokens[0];
            let amount_in = self.notional_in_token(start, dollars).await;

            handles.push(tokio::spawn(async move {
                let _permit = permit;

                let (p1_r0, p1_r1) = match snapshot.v2_get_reserves_at(route.pools[0], snap_block).await {
                    Ok(x) => x, Err(_) => return,
                };
                let (p2_r0, p2_r1) = match snapshot.v2_get_reserves_at(route.pools[1], snap_block).await {
                    Ok(x) => x, Err(_) => return,
                };

                let sim = match simulator.simulate_2hop_cycle_snapshot(
                    &route,
                    amount_in,
                    U256::from(p1_r0),
                    U256::from(p1_r1),
                    U256::from(p2_r0),
                    U256::from(p2_r1),
                    slippage_bps,
                    min_profit_usd,
                ).await {
                    Ok(s) => s, Err(_) => return,
                };

                if !sim.profitable { return; }

                let mut guard = best.lock().await;
                match &*guard {
                    Some((_, bsim)) if bsim.profit_usd >= sim.profit_usd => {}
                    _ => *guard = Some((route, sim)),
                }
            }));
        }

        for h in handles { let _ = h.await; }

        let guard = best.lock().await;
        Ok(guard.clone())
    }

    async fn evaluate_best_triangle(&self, snap_block: u64) -> Result<Option<(TriRoute, SimulationResult)>> {
        let stables = [Self::usdc_addr(), Self::usdt_addr(), Self::dai_addr()];
        let dollars = 100.0;

        self.decimals.preload(stables).await;

        let mut tris = self.routing.build_triangular_cycles(&self.chain.name);
        tris.retain(|t| self.universe.accept_route_tokens(&t.tokens));
        tris.retain(|t| t.tokens[0] == t.tokens[3] && stables.contains(&t.tokens[0]));
        tris.sort_by(|a,b| b.composite_price.partial_cmp(&a.composite_price).unwrap_or(std::cmp::Ordering::Equal));

        let top_k = Self::env_usize("SNAPSHOT_TOPK_TRI", 25).min(tris.len());
        let tris = tris.into_iter().take(top_k).collect::<Vec<_>>();
        if tris.is_empty() { return Ok(None); }

        let conc = Self::env_usize("SNAPSHOT_CONCURRENCY_TRI", 8).max(1);
        let sem = Arc::new(Semaphore::new(conc));
        let best: Arc<Mutex<Option<(TriRoute, SimulationResult)>>> = Arc::new(Mutex::new(None));
        let mut handles = Vec::with_capacity(tris.len());

        for tri in tris {
            let permit = sem.clone().acquire_owned().await?;
            let best = best.clone();
            let snapshot = self.snapshot.clone();
            let simulator = self.simulator.clone();
            let slippage_bps = self.chain.execution.slippage_bps;
            let min_profit_usd = self.chain.execution.min_profit_usd;

            let start = tri.tokens[0];
            let amount_in = self.notional_in_token(start, dollars).await;

            handles.push(tokio::spawn(async move {
                let _permit = permit;

                let (p1_r0, p1_r1) = match snapshot.v2_get_reserves_at(tri.pools[0], snap_block).await { Ok(x)=>x, Err(_)=>return };
                let (p2_r0, p2_r1) = match snapshot.v2_get_reserves_at(tri.pools[1], snap_block).await { Ok(x)=>x, Err(_)=>return };
                let (p3_r0, p3_r1) = match snapshot.v2_get_reserves_at(tri.pools[2], snap_block).await { Ok(x)=>x, Err(_)=>return };

                let sim = match simulator.simulate_triangle_snapshot(
                    &tri,
                    amount_in,
                    U256::from(p1_r0), U256::from(p1_r1),
                    U256::from(p2_r0), U256::from(p2_r1),
                    U256::from(p3_r0), U256::from(p3_r1),
                    slippage_bps,
                    min_profit_usd,
                ).await {
                    Ok(s)=>s, Err(_)=>return
                };

                if !sim.profitable { return; }

                let mut guard = best.lock().await;
                match &*guard {
                    Some((_, bsim)) if bsim.profit_usd >= sim.profit_usd => {}
                    _ => *guard = Some((tri, sim)),
                }
            }));
        }

        for h in handles { let _ = h.await; }
        let guard = best.lock().await;
        Ok(guard.clone())
    }

    pub async fn act(&self) -> Result<()> {
        if !self.chain.enabled {
            return Ok(());
        }

        let snap_block = self.snapshot.latest_block_number().await?;

        let best_2hop = self.evaluate_best_2hop(snap_block).await?;
        let best_tri = self.evaluate_best_triangle(snap_block).await?;

        match (best_2hop, best_tri) {
            (Some((r, s)), Some((t, ts))) => {
                if ts.profit_usd > s.profit_usd {
                    tracing::info!(
                        "[ARB] SNAPSHOT triangle {} block={} profit_usd={:.6} gas_usd={:.4} pools={:?} tokens={:?}",
                        self.chain.name, snap_block, ts.profit_usd, ts.gas_usd, t.pools, t.tokens
                    );
                } else {
                    tracing::info!(
                        "[ARB] SNAPSHOT 2-hop {} block={} profit_usd={:.6} gas_usd={:.4} pools={:?} tokens={:?}",
                        self.chain.name, snap_block, s.profit_usd, s.gas_usd, r.pools, r.tokens
                    );
                }
            }
            (Some((r, s)), None) => {
                tracing::info!(
                    "[ARB] SNAPSHOT 2-hop {} block={} profit_usd={:.6} gas_usd={:.4} pools={:?} tokens={:?}",
                    self.chain.name, snap_block, s.profit_usd, s.gas_usd, r.pools, r.tokens
                );
            }
            (None, Some((t, ts))) => {
                tracing::info!(
                    "[ARB] SNAPSHOT triangle {} block={} profit_usd={:.6} gas_usd={:.4} pools={:?} tokens={:?}",
                    self.chain.name, snap_block, ts.profit_usd, ts.gas_usd, t.pools, t.tokens
                );
            }
            (None, None) => {}
        }

        Ok(())
    }

    pub async fn evaluate_triangular(&self) -> Result<Option<(TriRoute, SimulationResult)>> {
        Ok(None)
    }
}
