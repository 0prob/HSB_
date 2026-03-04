use ethers::types::{Address, H256, U256};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Canonical pool identifier across chains.
/// This prevents collisions where the same 20-byte address exists on multiple chains.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PoolKey {
    pub chain_id: u64,
    pub pool: Address,
}

impl PartialEq for PoolKey {
    fn eq(&self, other: &Self) -> bool {
        self.chain_id == other.chain_id && self.pool == other.pool
    }
}
impl Eq for PoolKey {}

impl Hash for PoolKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.chain_id.hash(state);
        self.pool.hash(state);
    }
}

/// Unified swap event type across all DEXes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwapType {
    UniswapV2Swap,
    UniswapV2Sync,
    UniswapV3Swap,
    CurveExchange,
    BalancerSwap,
    AlgebraSwap,
    SushiSwap,
    QuickSwap,
}

/// Normalized swap event emitted by the HyperSync decoder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapEvent {
    pub chain_id: u64,
    pub chain: String,
    pub pool: Address,
    pub block_number: u64,
    pub tx_hash: H256,
    pub event_type: SwapType,

    // Token flow
    pub amount0_in: Option<U256>,
    pub amount1_in: Option<U256>,
    pub amount0_out: Option<U256>,
    pub amount1_out: Option<U256>,

    // For V2-style pools
    pub reserve0: Option<U256>,
    pub reserve1: Option<U256>,

    // For V3-style pools
    pub tick: Option<i32>,
    pub liquidity: Option<U256>,
}

/// Metadata for each discovered pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairMeta {
    pub chain_id: u64,
    pub chain: String,
    pub dex: String,
    pub pool: Address,
    pub token0: Address,
    pub token1: Address,
    pub fee_tier: Option<u32>,
}

/// Token metadata loaded from TOML configs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMeta {
    pub address: Address,
    pub decimals: u8,
    pub symbol: String,
}

/// Chain configuration loaded from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub enabled: bool,
    pub name: String,
    pub chain_id: u64,
    pub rpc_url: String,
    pub hypersync_url: String,
    pub hyperindex_url: String,

    pub gas: GasConfig,
    pub execution: ExecutionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasConfig {
    pub max_gwei: f64,
    pub priority_gwei: f64,
    pub block_time_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub executor_address: Address,
    pub slippage_bps: u32,
    pub min_profit_usd: f64,
    pub max_route_hops: usize,
}
