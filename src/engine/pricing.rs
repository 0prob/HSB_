use anyhow::Result;
use dashmap::DashMap;
use ethers::types::{Address, U256};
use std::sync::Arc;

use crate::types::SwapEvent;

/// Per‑pool price state.
#[derive(Debug, Clone)]
pub struct PoolPrice {
    pub price: f64,
    pub liquidity: Option<U256>,
    pub tick: Option<i32>,
}

/// Pricing engine maintains price state for all pools.
#[derive(Clone)]
pub struct PricingEngine {
    inner: Arc<DashMap<Address, PoolPrice>>,
}

impl PricingEngine {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    fn set_price(&self, pool: Address, price: f64, tick: Option<i32>, liq: Option<U256>) {
        self.inner.insert(pool, PoolPrice {
            price,
            liquidity: liq,
            tick,
        });
    }

    pub fn get_price(&self, pool: &Address) -> Option<PoolPrice> {
        self.inner.get(pool).map(|v| v.clone())
    }

    // -----------------------------
    // Uniswap V2 Pricing
    // -----------------------------
    pub async fn update_v2_swap(&self, ev: SwapEvent) -> Result<()> {
        if let (Some(a0), Some(a1)) = (ev.amount0_in, ev.amount1_out) {
            let price = a1.as_u128() as f64 / a0.as_u128() as f64;
            self.set_price(ev.pool, price, None, None);
        }
        Ok(())
    }

    pub async fn update_v2_sync(&self, ev: SwapEvent) -> Result<()> {
        if let (Some(r0), Some(r1)) = (ev.reserve0, ev.reserve1) {
            let p = r1.as_u128() as f64 / r0.as_u128() as f64;
            self.set_price(ev.pool, p, None, None);
        }
        Ok(())
    }

    // -----------------------------
    // Uniswap V3 Pricing
    // -----------------------------
    pub async fn update_v3_swap(&self, ev: SwapEvent) -> Result<()> {
        if let Some(tick) = ev.tick {
            let price = tick_to_price(tick);
            self.set_price(ev.pool, price, Some(ev.liquidity.unwrap_or_default()), Some(tick));
        }
        Ok(())
    }

    // -----------------------------
    // Curve Pricing
    // -----------------------------
    pub async fn update_curve(&self, ev: SwapEvent) -> Result<()> {
        if let (Some(sold), Some(bought)) = (ev.amount0_in, ev.amount0_out) {
            let price = bought.as_u128() as f64 / sold.as_u128() as f64;
            self.set_price(ev.pool, price, None, None);
        }
        Ok(())
    }

    // -----------------------------
    // Balancer Pricing
    // -----------------------------
    pub async fn update_balancer(&self, ev: SwapEvent) -> Result<()> {
        if let (Some(ain), Some(aout)) = (ev.amount0_in, ev.amount0_out) {
            let price = aout.as_u128() as f64 / ain.as_u128() as f64;
            self.set_price(ev.pool, price, None, None);
        }
        Ok(())
    }

    // -----------------------------
    // Algebra/Maverick Pricing
    // -----------------------------
    pub async fn update_algebra(&self, ev: SwapEvent) -> Result<()> {
        if let Some(tick) = ev.tick {
            let price = tick_to_price(tick);
            self.set_price(ev.pool, price, Some(ev.liquidity.unwrap_or_default()), Some(tick));
        }
        Ok(())
    }
}

/// Convert a Uniswap‑V3‑style tick into a price.
/// price = 1.0001^tick
fn tick_to_price(tick: i32) -> f64 {
    const BASE: f64 = 1.0001;
    BASE.powi(tick)
}
