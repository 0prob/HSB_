use anyhow::Result;
use dashmap::DashMap;
use ethers::types::{Address, U256};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use crate::types::{PoolKey, SwapEvent};

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
    inner: Arc<DashMap<PoolKey, PoolPrice>>,
    latest_block_by_chain: Arc<DashMap<u64, AtomicU64>>,
}

impl PricingEngine {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
            latest_block_by_chain: Arc::new(DashMap::new()),
        }
    }

    fn bump_latest_block(&self, chain_id: u64, bn: u64) {
        let entry = self
            .latest_block_by_chain
            .entry(chain_id)
            .or_insert_with(|| AtomicU64::new(0));
        let cur = entry.load(Ordering::Relaxed);
        if bn > cur {
            entry.store(bn, Ordering::Relaxed);
        }
    }

    pub fn latest_observed_block(&self, chain_id: u64) -> u64 {
        self.latest_block_by_chain
            .get(&chain_id)
            .map(|v| v.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    fn upsert(&self, key: PoolKey, mut f: impl FnMut(&mut PoolPrice)) {
        self.inner
            .entry(key)
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

    pub fn get_price(&self, chain_id: u64, pool: &Address) -> Option<PoolPrice> {
        let key = PoolKey {
            chain_id,
            pool: *pool,
        };
        self.inner.get(&key).map(|v| v.clone())
    }

    pub fn get_v2_reserves(&self, chain_id: u64, pool: &Address) -> Option<(U256, U256)> {
        let key = PoolKey {
            chain_id,
            pool: *pool,
        };
        let p = self.inner.get(&key)?;
        Some((p.reserve0?, p.reserve1?))
    }

    pub fn get_v2_last_sync_block(&self, chain_id: u64, pool: &Address) -> Option<u64> {
        let key = PoolKey {
            chain_id,
            pool: *pool,
        };
        let p = self.inner.get(&key)?;
        p.last_sync_block
    }

    // V2: ignore Swap for pricing; use Sync only.
    pub async fn update_v2_swap(&self, ev: SwapEvent) -> Result<()> {
        self.bump_latest_block(ev.chain_id, ev.block_number);
        Ok(())
    }

    pub async fn update_v2_sync(&self, ev: SwapEvent) -> Result<()> {
        self.bump_latest_block(ev.chain_id, ev.block_number);

        if let (Some(r0), Some(r1)) = (ev.reserve0, ev.reserve1) {
            if r0.is_zero() || r1.is_zero() {
                return Ok(());
            }

            // NOTE: this is still a raw reserve ratio (not decimals-normalized).
            // The refactor here is about correctness of multi-chain keying.
            // A subsequent patch should normalize using token decimals.
            let r0f = u256_to_f64_clamped(r0);
            let r1f = u256_to_f64_clamped(r1);
            if r0f <= 0.0 || r1f <= 0.0 {
                return Ok(());
            }
            let p = r1f / r0f;
            if !(1e-12..=1e12).contains(&p) {
                return Ok(());
            }

            let bn = ev.block_number;
            let key = PoolKey {
                chain_id: ev.chain_id,
                pool: ev.pool,
            };

            self.upsert(key, |st| {
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
        self.bump_latest_block(ev.chain_id, ev.block_number);

        if let Some(tick) = ev.tick {
            let price = tick_to_price(tick);
            if !(1e-12..=1e12).contains(&price) {
                return Ok(());
            }
            let key = PoolKey {
                chain_id: ev.chain_id,
                pool: ev.pool,
            };
            self.upsert(key, |st| {
                st.price = price;
                st.tick = Some(tick);
                st.liquidity = ev.liquidity;
            });
        }
        Ok(())
    }

    pub async fn update_curve(&self, ev: SwapEvent) -> Result<()> {
        self.bump_latest_block(ev.chain_id, ev.block_number);
        if let (Some(sold), Some(bought)) = (ev.amount0_in, ev.amount0_out) {
            if sold.is_zero() {
                return Ok(());
            }
            let soldf = u256_to_f64_clamped(sold);
            let boughtf = u256_to_f64_clamped(bought);
            if soldf <= 0.0 || boughtf <= 0.0 {
                return Ok(());
            }
            let price = boughtf / soldf;
            if !(1e-12..=1e12).contains(&price) {
                return Ok(());
            }
            let key = PoolKey {
                chain_id: ev.chain_id,
                pool: ev.pool,
            };
            self.upsert(key, |st| st.price = price);
        }
        Ok(())
    }

    pub async fn update_balancer(&self, ev: SwapEvent) -> Result<()> {
        self.bump_latest_block(ev.chain_id, ev.block_number);
        if let (Some(ain), Some(aout)) = (ev.amount0_in, ev.amount0_out) {
            if ain.is_zero() {
                return Ok(());
            }
            let ainf = u256_to_f64_clamped(ain);
            let aoutf = u256_to_f64_clamped(aout);
            if ainf <= 0.0 || aoutf <= 0.0 {
                return Ok(());
            }
            let price = aoutf / ainf;
            if !(1e-12..=1e12).contains(&price) {
                return Ok(());
            }
            let key = PoolKey {
                chain_id: ev.chain_id,
                pool: ev.pool,
            };
            self.upsert(key, |st| st.price = price);
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

/// Lossy-but-safe-ish conversion with range clamp.
/// This avoids silent truncation via `as_u128()` and keeps the engine stable.
/// Follow-up refactor should move to rational math + decimals normalization.
fn u256_to_f64_clamped(x: U256) -> f64 {
    let mut buf = [0u8; 32];
    x.to_big_endian(&mut buf);

    // Convert high 16 bytes and low 16 bytes as u128 to build an f64.
    let hi = u128::from_be_bytes(buf[0..16].try_into().unwrap());
    let lo = u128::from_be_bytes(buf[16..32].try_into().unwrap());

    // If hi is nonzero, x is > 2^128 and precision will be poor; clamp to a large value.
    if hi != 0 {
        // large sentinel; pricing code already bounds accepted ratios
        return 1.0e38;
    }
    lo as f64
}
