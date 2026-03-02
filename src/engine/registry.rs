use dashmap::DashMap;
use ethers::types::Address;
use std::sync::Arc;

use crate::types::PairMeta;

/// Central registry of all discovered pools across all chains.
/// Updated by HyperIndex discovery and read by HyperSync subscriber.
#[derive(Clone)]
pub struct PairRegistry {
    inner: Arc<DashMap<Address, PairMeta>>,
}

impl PairRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }

    /// Insert or update metadata for a pool.
    pub fn insert(&self, meta: PairMeta) {
        self.inner.insert(meta.pool, meta);
    }

    /// Remove a pool (rarely used, but included for completeness).
    pub fn remove(&self, pool: &Address) {
        self.inner.remove(pool);
    }

    /// Get metadata for a pool.
    pub fn get(&self, pool: &Address) -> Option<PairMeta> {
        self.inner.get(pool).map(|v| v.clone())
    }

    /// Return all pool addresses for HyperSync filtering.
    pub fn all_addresses(&self) -> Vec<Address> {
        self.inner.iter().map(|kv| *kv.key()).collect()
    }

    /// Return all pools for a specific chain.
    pub fn by_chain(&self, chain: &str) -> Vec<PairMeta> {
        self.inner
            .iter()
            .filter(|kv| kv.value().chain == chain)
            .map(|kv| kv.value().clone())
            .collect()
    }

    /// Return all pools for a specific DEX.
    pub fn by_dex(&self, dex: &str) -> Vec<PairMeta> {
        self.inner
            .iter()
            .filter(|kv| kv.value().dex == dex)
            .map(|kv| kv.value().clone())
            .collect()
    }
}
