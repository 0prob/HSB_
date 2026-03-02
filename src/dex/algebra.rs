use anyhow::Result;
use ethers::abi::{AbiDecode, RawLog};
use ethers::types::{Log, U256};

use crate::types::{SwapEvent, SwapType, PairMeta};

/// Algebra/Maverick Swap event:
/// Swap(address sender, address recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
pub fn decode_swap(
    chain: String,
    meta: &PairMeta,
    log: &Log,
    raw: RawLog,
) -> Result<SwapEvent> {
    let decoded: (
        ethers::types::Address,
        ethers::types::Address,
        i128,
        i128,
        u128,
        u128,
        i32
    ) = AbiDecode::decode(raw.data.as_ref())?;

    let (amount0_in, amount0_out) = if decoded.2 > 0 {
        (Some(U256::from(decoded.2 as u128)), None)
    } else {
        (None, Some(U256::from((-decoded.2) as u128)))
    };

    let (amount1_in, amount1_out) = if decoded.3 > 0 {
        (Some(U256::from(decoded.3 as u128)), None)
    } else {
        (None, Some(U256::from((-decoded.3) as u128)))
    };

    Ok(SwapEvent {
        chain,
        pool: meta.pool,
        block_number: log.block_number.unwrap().as_u64(),
        tx_hash: log.transaction_hash.unwrap(),
        event_type: SwapType::AlgebraSwap,

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

pub async fn handle_swap(ev: SwapEvent) -> Result<()> {
    tracing::debug!(
        "[Algebra Swap] pool={} tick={} liquidity={}",
        ev.pool,
        ev.tick.unwrap_or_default(),
        ev.liquidity.unwrap_or_default()
    );

    // TODO: integrate with pricing engine
    Ok(())
}
