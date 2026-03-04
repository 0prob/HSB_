use anyhow::Result;
use hypersync_client::net_types::LogFilter;

use crate::engine::registry::PairRegistry;
use crate::types::ChainConfig;

/// Build a HyperSync LogFilter for the current registry pools.
pub fn build_filter(chain: &ChainConfig, registry: &PairRegistry) -> Result<LogFilter> {
    let addrs = registry
        .addresses_by_chain_id(chain.chain_id)
        .into_iter()
        .map(|a| format!("{:#x}", a))
        .collect::<Vec<_>>();

    let topics0 = vec![
        // UniswapV2 Swap
        format!(
            "0x{}",
            ethers::utils::hex::encode(ethers::utils::keccak256(
                "Swap(address,uint256,uint256,uint256,uint256,address)".as_bytes()
            ))
        ),
        // UniswapV2 Sync
        format!(
            "0x{}",
            ethers::utils::hex::encode(ethers::utils::keccak256("Sync(uint112,uint112)".as_bytes()))
        ),
        // UniswapV3/Algebra Swap signature (same)
        format!(
            "0x{}",
            ethers::utils::hex::encode(ethers::utils::keccak256(
                "Swap(address,address,int256,int256,uint160,uint128,int24)".as_bytes()
            ))
        ),
        // Curve TokenExchange (common)
        format!(
            "0x{}",
            ethers::utils::hex::encode(ethers::utils::keccak256(
                "TokenExchange(address,int128,uint256,int128,uint256)".as_bytes()
            ))
        ),
        // Balancer Swap (Vault event)
        format!(
            "0x{}",
            ethers::utils::hex::encode(ethers::utils::keccak256(
                "Swap(address,address,address,uint256,uint256)".as_bytes()
            ))
        ),
    ];

    let mut f = LogFilter::all();
    if !addrs.is_empty() {
        f = f.and_address(addrs)?;
    }
    f = f.and_topic0(topics0)?;
    Ok(f)
}
