use hypersync_client::net_types::LogFilter;
use crate::engine::registry::PairRegistry;
use crate::types::ChainConfig;

/// Build a dynamic HyperSync filter for all pools on a chain.
/// Includes all DEX event signatures relevant to this engine.
pub fn build_filter(chain: &ChainConfig, registry: &PairRegistry) -> LogFilter {
    let addrs: Vec<String> = registry
        .by_chain(&chain.name)
        .iter()
        .map(|p| format!("{:?}", p.pool))
        .collect();

    // Event signatures for all supported DEXes.
    // These are topic0 values.
    let topics = vec![vec![
        // Uniswap V2 Swap
        format!("{:?}", ethers::utils::id("Swap(address,uint256,uint256,uint256,uint256,address)")),
        // Uniswap V2 Sync
        format!("{:?}", ethers::utils::id("Sync(uint112,uint112)")),
        // Uniswap V3 Swap
        format!("{:?}", ethers::utils::id("Swap(address,address,int256,int256,uint160,uint128,int24)")),
        // Curve Exchange
        format!("{:?}", ethers::utils::id("TokenExchange(address,int128,uint256,int128,uint256)")),
        // Balancer Swap
        format!("{:?}", ethers::utils::id("Swap(address,address,address,uint256,uint256)")),
        // Algebra/Maverick Swap
        format!("{:?}", ethers::utils::id("Swap(address,address,int256,int256,uint160,uint128,int24)")),
    ]];

    LogFilter {
        addresses: Some(addrs),
        topics: Some(topics),
        ..Default::default()
    }
}
