use anyhow::Result;
use dashmap::DashMap;
use ethers::types::{Address, U256};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use crate::types::SwapEvent;

#[derive(Debug, Clone)]
pub struct PoolPrice {
    pub price: f64,

    // V2 reserves
    pub reserve0: Option<U256>,
    pub reserve1: Option<U256>,
    pub last_sync_block: Option<u64>,

    // V3-style
    pub tick: Option<i32>,
    pub liquidity: Option<U256>,
}

#[derive(Clone)]
pub struct PricingEngine {
    inner: Arc<DashMap<Address, PoolPrice>>,
    latest_block: Arc<AtomicU64>,
}

impl PricingEngine {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
            latest_block: Arc::new(AtomicU64::new(0)),
        }
    }

    fn bump_latest_block(&self, bn: u64) {
        let cur = self.latest_block.load(Ordering::Relaxed);
        if bn > cur {
            self.latest_block.store(bn, Ordering::Relaxed);
        }
    }

    pub fn latest_observed_block(&self) -> u64 {
        self.latest_block.load(Ordering::Relaxed)
    }

    fn upsert(&self, pool: Address, mut f: impl FnMut(&mut PoolPrice)) {
        self.inner
            .entry(pool)
            .and_modify(|v| f(v))
            .or_insert_with(|| {
                let mut v = PoolPrice {
                    price: 0.0,
                    reserve0: None,
                    reserve1: None,
                    last_sync_block: None,
                    tick: None,
                    liquidity: None,
                };
                f(&mut v);
                v
            });
    }

    pub fn get_price(&self, pool: &Address) -> Option<PoolPrice> {
        self.inner.get(pool).map(|v| v.clone())
    }

    pub fn get_v2_reserves(&self, pool: &Address) -> Option<(U256, U256)> {
        let p = self.inner.get(pool)?;
        Some((p.reserve0?, p.reserve1?))
    }

    pub fn get_v2_last_sync_block(&self, pool: &Address) -> Option<u64> {
        let p = self.inner.get(pool)?;
        p.last_sync_block
    }

    // V2: ignore Swap for pricing; use Sync only.
    pub async fn update_v2_swap(&self, ev: SwapEvent) -> Result<()> {
        self.bump_latest_block(ev.block_number);
        Ok(())
    }

    pub async fn update_v2_sync(&self, ev: SwapEvent) -> Result<()> {
        self.bump_latest_block(ev.block_number);

        if let (Some(r0), Some(r1)) = (ev.reserve0, ev.reserve1) {
            if r0.is_zero() || r1.is_zero() {
                return Ok(());
            }
            let p = r1.as_u128() as f64 / r0.as_u128() as f64;
            if !(1e-12..=1e12).contains(&p) {
                return Ok(());
            }
            let bn = ev.block_number;
            self.upsert(ev.pool, |st| {
                st.price = p;
                st.reserve0 = Some(r0);
                st.reserve1 = Some(r1);
                st.last_sync_block = Some(bn);
                st.tick = None;
                st.liquidity = None;
            });
        }
        Ok(())
    }

    pub async fn update_v3_swap(&self, ev: SwapEvent) -> Result<()> {
        self.bump_latest_block(ev.block_number);
        if let Some(tick) = ev.tick {
            let price = tick_to_price(tick);
            if !(1e-12..=1e12).contains(&price) {
                return Ok(());
            }
            self.upsert(ev.pool, |st| {
                st.price = price;
                st.tick = Some(tick);
                st.liquidity = ev.liquidity;
            });
        }
        Ok(())
    }

    pub async fn update_curve(&self, ev: SwapEvent) -> Result<()> {
        self.bump_latest_block(ev.block_number);
        if let (Some(sold), Some(bought)) = (ev.amount0_in, ev.amount0_out) {
            if sold.is_zero() {
                return Ok(());
            }
            let price = bought.as_u128() as f64 / sold.as_u128() as f64;
            if !(1e-12..=1e12).contains(&price) {
                return Ok(());
            }
            self.upsert(ev.pool, |st| st.price = price);
        }
        Ok(())
    }

    pub async fn update_balancer(&self, ev: SwapEvent) -> Result<()> {
        self.bump_latest_block(ev.block_number);
        if let (Some(ain), Some(aout)) = (ev.amount0_in, ev.amount0_out) {
            if ain.is_zero() {
                return Ok(());
            }
            let price = aout.as_u128() as f64 / ain.as_u128() as f64;
            if !(1e-12..=1e12).contains(&price) {
                return Ok(());
            }
            self.upsert(ev.pool, |st| st.price = price);
        }
        Ok(())
    }

    pub async fn update_algebra(&self, ev: SwapEvent) -> Result<()> {
        // treat as V3-style for pricing purposes
        self.update_v3_swap(ev).await
    }
}

fn tick_to_price(tick: i32) -> f64 {
    const BASE: f64 = 1.0001;
    BASE.powi(tick)
}
