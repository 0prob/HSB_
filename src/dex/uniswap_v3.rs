use anyhow::Result;
use ethers::abi::{AbiDecode, RawLog};
use ethers::types::{Log, U256};

use crate::types::{SwapEvent, SwapType, PairMeta};

/// Decode a Uniswap V3 Swap event.
pub fn decode_swap(
    chain: String,
    meta: &PairMeta,
    log: &Log,
    raw: RawLog,
) -> Result<SwapEvent> {
    // Swap(address,address,int256,int256,uint160,uint128,int24)
    let decoded: (
        ethers::types::Address,
        ethers::types::Address,
        i128,
        i128,
        u128,
        u128,
        i32,
    ) = AbiDecode::decode(raw.data.as_ref())?;

    let amount0 = decoded.2;
    let amount1 = decoded.3;

    let (amount0_in, amount0_out) = if amount0 > 0 {
        (Some(U256::from(amount0 as u128)), None)
    } else {
        (None, Some(U256::from((-amount0) as u128)))
    };

    let (amount1_in, amount1_out) = if amount1 > 0 {
        (Some(U256::from(amount1 as u128)), None)
    } else {
        (None, Some(U256::from((-amount1) as u128)))
    };

    Ok(SwapEvent {
        chain,
        pool: meta.pool,
        block_number: log.block_number.unwrap().as_u64(),
        tx_hash: log.transaction_hash.unwrap(),
        event_type: SwapType::UniswapV3Swap,

        amount0_in,
        amount1_in,
        amount0_out,
        amount1_out,

        reserve0: None,
        reserve1: None,
        tick: Some(decoded.6),
        liquidity: Some(U256::from(decoded.5)),
    })
}

/// Handle a normalized V3 swap event.
pub async fn handle_swap(ev: SwapEvent) -> Result<()> {
    tracing::debug!(
        "[V3 Swap] pool={} tick={} liquidity={}",
        ev.pool,
        ev.tick.unwrap_or_default(),
        ev.liquidity.unwrap_or_default()
    );

    // TODO: integrate with pricing engine
    Ok(())
}
