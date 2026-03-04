use anyhow::{anyhow, Result};
use ethers::types::{Address, U256};

use crate::engine::decimals::DecimalsCache;
use crate::engine::gas::GasModel;
use crate::engine::registry::PairRegistry;
use crate::engine::routing::{Route, TriRoute};

#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub profitable: bool,
    pub profit_usd: f64,
    pub gas_usd: f64,
}

#[derive(Clone)]
pub struct ProfitSimulator {
    gas: GasModel,
    registry: PairRegistry,
    decimals: DecimalsCache,
}

impl ProfitSimulator {
    pub fn new(gas: GasModel, registry: PairRegistry, _pricing: crate::engine::pricing::PricingEngine, decimals: DecimalsCache) -> Self {
        Self { gas, registry, decimals }
    }

    fn v2_swap_out(amount_in: U256, reserve_in: U256, reserve_out: U256) -> Option<U256> {
        if amount_in.is_zero() || reserve_in.is_zero() || reserve_out.is_zero() {
            return None;
        }
        let fee_mul = U256::from(997u64);
        let fee_den = U256::from(1000u64);

        let amount_in_with_fee = amount_in.checked_mul(fee_mul)?;
        let numerator = amount_in_with_fee.checked_mul(reserve_out)?;
        let denominator = reserve_in.checked_mul(fee_den)?.checked_add(amount_in_with_fee)?;
        numerator.checked_div(denominator)
    }

    fn stable_kind(token: Address) -> bool {
        let usdc: Address = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse().unwrap();
        let usdt: Address = "0xC2132D05D31c914a87C6611C10748AEb04B58e8F".parse().unwrap();
        let dai:  Address = "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063".parse().unwrap();
        token == usdc || token == usdt || token == dai
    }

    async fn token_amount_to_usd(&self, token: Address, amt: U256) -> f64 {
        if !Self::stable_kind(token) {
            return 0.0;
        }
        let dec = self.decimals.get_or_fetch(token).await.unwrap_or(18);
        let denom = 10f64.powi(dec as i32);
        (amt.as_u128() as f64) / denom
    }

    fn orient_reserves(&self, pool: Address, token_in: Address, token_out: Address, reserve0: U256, reserve1: U256) -> Option<(U256, U256)> {
        let meta = self.registry.get(&pool)?;
        if token_in == meta.token0 && token_out == meta.token1 {
            Some((reserve0, reserve1))
        } else if token_in == meta.token1 && token_out == meta.token0 {
            Some((reserve1, reserve0))
        } else {
            None
        }
    }

    fn require_liquidity(reserve_in: U256, amount_in: U256) -> bool {
        let mult: U256 = std::env::var("MIN_RESERVE_IN_MULT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(U256::from)
            .unwrap_or_else(|| U256::from(50u64));

        match amount_in.checked_mul(mult) {
            Some(th) => reserve_in >= th,
            None => false,
        }
    }

    pub async fn simulate_2hop_cycle_snapshot(
        &self,
        route: &Route,
        amount_in: U256,
        p1_r0: U256,
        p1_r1: U256,
        p2_r0: U256,
        p2_r1: U256,
        slippage_bps: u32,
        min_profit_usd: f64,
    ) -> Result<SimulationResult> {
        if route.tokens.len() != 3 || route.tokens[0] != route.tokens[2] || route.pools.len() != 2 {
            return Err(anyhow!("route is not a 2-hop cycle"));
        }

        let a = route.tokens[0];
        let b = route.tokens[1];

        let (r_in1, r_out1) = self.orient_reserves(route.pools[0], a, b, p1_r0, p1_r1)
            .ok_or_else(|| anyhow!("cannot orient hop1 reserves"))?;

        if !Self::require_liquidity(r_in1, amount_in) {
            return Ok(SimulationResult { profitable: false, profit_usd: 0.0, gas_usd: 0.0 });
        }

        let out1 = Self::v2_swap_out(amount_in, r_in1, r_out1).ok_or_else(|| anyhow!("swap math failed hop1"))?;

        let (r_in2, r_out2) = self.orient_reserves(route.pools[1], b, a, p2_r0, p2_r1)
            .ok_or_else(|| anyhow!("cannot orient hop2 reserves"))?;

        if !Self::require_liquidity(r_in2, out1) {
            return Ok(SimulationResult { profitable: false, profit_usd: 0.0, gas_usd: 0.0 });
        }

        let out2 = Self::v2_swap_out(out1, r_in2, r_out2).ok_or_else(|| anyhow!("swap math failed hop2"))?;

        // Slippage haircut
        let slip = U256::from(slippage_bps);
        let denom = U256::from(10_000u64);
        let slip_amt = out2.checked_mul(slip).unwrap_or(U256::zero()).checked_div(denom).unwrap_or(U256::zero());
        let out2_adj = out2.saturating_sub(slip_amt);

        let profit_token = out2_adj.saturating_sub(amount_in);

        let gas_usd = self.gas.estimate_route_cost(route)?;
        let profit_usd_gross = self.token_amount_to_usd(a, profit_token).await;
        let profit_usd = profit_usd_gross - gas_usd;

        let profitable = Self::stable_kind(a) && profit_usd >= min_profit_usd;

        Ok(SimulationResult { profitable, profit_usd, gas_usd })
    }

    pub async fn simulate_triangle_snapshot(
        &self,
        tri: &TriRoute,
        amount_in: U256,
        p1_r0: U256,
        p1_r1: U256,
        p2_r0: U256,
        p2_r1: U256,
        p3_r0: U256,
        p3_r1: U256,
        slippage_bps: u32,
        min_profit_usd: f64,
    ) -> Result<SimulationResult> {
        // tokens [A,B,C,A]
        let a = tri.tokens[0];
        let b = tri.tokens[1];
        let c = tri.tokens[2];

        let (r_in1, r_out1) = self.orient_reserves(tri.pools[0], a, b, p1_r0, p1_r1)
            .ok_or_else(|| anyhow!("cannot orient hop1 reserves"))?;
        if !Self::require_liquidity(r_in1, amount_in) {
            return Ok(SimulationResult { profitable: false, profit_usd: 0.0, gas_usd: 0.0 });
        }
        let out1 = Self::v2_swap_out(amount_in, r_in1, r_out1).ok_or_else(|| anyhow!("swap math failed hop1"))?;

        let (r_in2, r_out2) = self.orient_reserves(tri.pools[1], b, c, p2_r0, p2_r1)
            .ok_or_else(|| anyhow!("cannot orient hop2 reserves"))?;
        if !Self::require_liquidity(r_in2, out1) {
            return Ok(SimulationResult { profitable: false, profit_usd: 0.0, gas_usd: 0.0 });
        }
        let out2 = Self::v2_swap_out(out1, r_in2, r_out2).ok_or_else(|| anyhow!("swap math failed hop2"))?;

        let (r_in3, r_out3) = self.orient_reserves(tri.pools[2], c, a, p3_r0, p3_r1)
            .ok_or_else(|| anyhow!("cannot orient hop3 reserves"))?;
        if !Self::require_liquidity(r_in3, out2) {
            return Ok(SimulationResult { profitable: false, profit_usd: 0.0, gas_usd: 0.0 });
        }
        let out3 = Self::v2_swap_out(out2, r_in3, r_out3).ok_or_else(|| anyhow!("swap math failed hop3"))?;

        let slip = U256::from(slippage_bps);
        let denom = U256::from(10_000u64);
        let slip_amt = out3.checked_mul(slip).unwrap_or(U256::zero()).checked_div(denom).unwrap_or(U256::zero());
        let out3_adj = out3.saturating_sub(slip_amt);

        let profit_token = out3_adj.saturating_sub(amount_in);

        // Gas cost: approximate 3-hop by a pseudo Route
        let pseudo = Route { pools: vec![tri.pools[0], tri.pools[1], tri.pools[2]], tokens: vec![a,b,c,a], price: tri.composite_price };
        let gas_usd = self.gas.estimate_route_cost(&pseudo)?;

        let profit_usd_gross = self.token_amount_to_usd(a, profit_token).await;
        let profit_usd = profit_usd_gross - gas_usd;

        let profitable = Self::stable_kind(a) && profit_usd >= min_profit_usd;

        Ok(SimulationResult { profitable, profit_usd, gas_usd })
    }

    pub async fn simulate_triangular(
        &self,
        _tri: &TriRoute,
        _amount_in_usd: f64,
        _slippage_bps: u32,
        _min_profit_usd: f64,
    ) -> Result<SimulationResult> {
        Ok(SimulationResult { profitable: false, profit_usd: 0.0, gas_usd: 0.0 })
    }
}
