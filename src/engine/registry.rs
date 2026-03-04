use dashmap::DashMap;
use ethers::types::Address;
use std::sync::Arc;

use crate::types::{PairMeta, PoolKey};

/// Central registry of all discovered pools across all chains.
/// Keyed by (chain_id, pool_address) to prevent cross-chain collisions.
#[derive(Clone)]
pub struct PairRegistry {
    inner: Arc<DashMap<PoolKey, PairMeta>>,
}

impl PairRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    /// Insert or update metadata for a pool.
    pub fn insert(&self, meta: PairMeta) {
        let key = PoolKey {
            chain_id: meta.chain_id,
            pool: meta.pool,
        };
        self.inner.insert(key, meta);
    }

    /// Remove a pool.
    pub fn remove(&self, chain_id: u64, pool: &Address) {
        let key = PoolKey {
            chain_id,
            pool: *pool,
        };
        self.inner.remove(&key);
    }

    /// Get metadata for a pool on a specific chain.
    pub fn get(&self, chain_id: u64, pool: &Address) -> Option<PairMeta> {
        let key = PoolKey {
            chain_id,
            pool: *pool,
        };
        self.inner.get(&key).map(|v| v.clone())
    }

    /// Return all pools for a specific chain_id.
    pub fn by_chain_id(&self, chain_id: u64) -> Vec<PairMeta> {
        self.inner
            .iter()
            .filter(|kv| kv.key().chain_id == chain_id)
            .map(|kv| kv.value().clone())
            .collect()
    }

    /// Return all pools for a specific chain name (UI-only; do not use for correctness).
    pub fn by_chain_name(&self, chain: &str) -> Vec<PairMeta> {
        self.inner
            .iter()
            .filter(|kv| kv.value().chain == chain)
            .map(|kv| kv.value().clone())
            .collect()
    }

    /// Return all pool addresses for a specific chain_id (for HyperSync address filters).
    pub fn addresses_by_chain_id(&self, chain_id: u64) -> Vec<Address> {
        self.inner
            .iter()
            .filter(|kv| kv.key().chain_id == chain_id)
            .map(|kv| kv.key().pool)
            .collect()
    }
}
