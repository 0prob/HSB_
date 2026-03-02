use anyhow::Result;
use ethers::abi::{AbiDecode, RawLog};
use ethers::types::{Log, U256};

use crate::types::{SwapEvent, SwapType, PairMeta};

/// Decode a Uniswap V2 Swap event.
pub fn decode_swap(
    chain: String,
    meta: &PairMeta,
    log: &Log,
    raw: RawLog,
) -> Result<SwapEvent> {
    // Swap(address,uint256,uint256,uint256,uint256,address)
    let decoded: (ethers::types::Address, U256, U256, U256, U256, ethers::types::Address) =
        AbiDecode::decode(raw.data.as_ref())?;

    Ok(SwapEvent {
        chain,
        pool: meta.pool,
        block_number: log.block_number.unwrap().as_u64(),
        tx_hash: log.transaction_hash.unwrap(),
        event_type: SwapType::UniswapV2Swap,

        amount0_in: Some(decoded.1),
        amount1_in: Some(decoded.2),
        amount0_out: Some(decoded.3),
        amount1_out: Some(decoded.4),

        reserve0: None,
        reserve1: None,
        tick: None,
        liquidity: None,
    })
}

/// Decode a Uniswap V2 Sync event.
pub fn decode_sync(
    chain: String,
    meta: &PairMeta,
    log: &Log,
    raw: RawLog,
) -> Result<SwapEvent> {
    // Sync(uint112,uint112)
    let decoded: (U256, U256) = AbiDecode::decode(raw.data.as_ref())?;

    Ok(SwapEvent {
        chain,
        pool: meta.pool,
        block_number: log.block_number.unwrap().as_u64(),
        tx_hash: log.transaction_hash.unwrap(),
        event_type: SwapType::UniswapV2Sync,

        amount0_in: None,
        amount1_in: None,
        amount0_out: None,
        amount1_out: None,

        reserve0: Some(decoded.0),
        reserve1: Some(decoded.1),
        tick: None,
        liquidity: None,
    })
}

/// Handle a normalized V2 swap event.
/// This is where you forward into pricing/arb logic.
pub async fn handle_swap(ev: SwapEvent) -> Result<()> {
    tracing::debug!(
        "[V2 Swap] pool={} amount0_in={} amount1_in={}",
        ev.pool,
        ev.amount0_in.unwrap_or_default(),
        ev.amount1_in.unwrap_or_default()
    );

    // TODO: integrate with pricing engine
    Ok(())
}

/// Handle a normalized V2 sync event.
pub async fn handle_sync(ev: SwapEvent) -> Result<()> {
    tracing::debug!(
        "[V2 Sync] pool={} reserve0={} reserve1={}",
        ev.pool,
        ev.reserve0.unwrap_or_default(),
        ev.reserve1.unwrap_or_default()
    );

    // TODO: integrate with pricing engine
    Ok(())
}
