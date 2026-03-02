use anyhow::Result;
use ethers::abi::RawLog;
use ethers::types::Log;

use crate::engine::registry::PairRegistry;
use crate::types::{SwapEvent, SwapType};

/// Decode a raw HyperSync log into a normalized SwapEvent.
/// Dispatches to the correct DEX decoder based on topic0.
pub async fn decode_log(
    chain: String,
    log: Log,
    registry: PairRegistry,
) -> Result<()> {
    let topic0 = match log.topics.get(0) {
        Some(t) => *t,
        None => return Ok(()),
    };

    let pool = log.address;

    let meta = match registry.get(&pool) {
        Some(m) => m,
        None => return Ok(()), // unknown pool
    };

    let raw = RawLog {
        topics: log.topics.clone(),
        data: log.data.to_vec(),
    };

    // Uniswap V2 Swap
    if topic0 == ethers::utils::id("Swap(address,uint256,uint256,uint256,uint256,address)").into() {
        let ev = super::dex_uniswap_v2::decode_swap(chain, &meta, &log, raw)?;
        return super::dex_uniswap_v2::handle_swap(ev).await;
    }

    // Uniswap V2 Sync
    if topic0 == ethers::utils::id("Sync(uint112,uint112)").into() {
        let ev = super::dex_uniswap_v2::decode_sync(chain, &meta, &log, raw)?;
        return super::dex_uniswap_v2::handle_sync(ev).await;
    }

    // Uniswap V3 Swap
    if topic0 == ethers::utils::id("Swap(address,address,int256,int256,uint160,uint128,int24)").into() {
        let ev = super::dex_uniswap_v3::decode_swap(chain, &meta, &log, raw)?;
        return super::dex_uniswap_v3::handle_swap(ev).await;
    }

    // Curve Exchange
    if topic0 == ethers::utils::id("TokenExchange(address,int128,uint256,int128,uint256)").into() {
        let ev = super::dex_curve::decode_exchange(chain, &meta, &log, raw)?;
        return super::dex_curve::handle_exchange(ev).await;
    }

    // Balancer Swap
    if topic0 == ethers::utils::id("Swap(address,address,address,uint256,uint256)").into() {
        let ev = super::dex_balancer::decode_swap(chain, &meta, &log, raw)?;
        return super::dex_balancer::handle_swap(ev).await;
    }

    // Algebra/Maverick Swap
    if topic0 == ethers::utils::id("Swap(address,address,int256,int256,uint160,uint128,int24)").into() {
        let ev = super::dex_algebra::decode_swap(chain, &meta, &log, raw)?;
        return super::dex_algebra::handle_swap(ev).await;
    }

    Ok(())
}
