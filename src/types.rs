use ethers::types::{Address, H256, U256};
use serde::{Deserialize, Serialize};

/// Unified swap event type across all DEXes.
/// This allows the engine to normalize Uniswap V2, V3, Curve, Balancer, Algebra, etc.
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
/// Every DEX event is converted into this unified structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapEvent {
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
/// Populated by HyperIndex discovery and enriched by on-chain queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairMeta {
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
