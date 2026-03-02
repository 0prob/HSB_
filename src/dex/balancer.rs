use anyhow::Result;
use ethers::abi::{AbiDecode, RawLog};
use ethers::types::{Log, U256};

use crate::types::{SwapEvent, SwapType, PairMeta};

/// Balancer V2 Swap event:
/// Swap(address poolId, address tokenIn, address tokenOut, uint256 amountIn, uint256 amountOut)
pub fn decode_swap(
    chain: String,
    meta: &PairMeta,
    log: &Log,
    raw: RawLog,
) -> Result<SwapEvent> {
    let decoded: (
        ethers::types::Address,
        ethers::types::Address,
        ethers::types::Address,
        U256,
        U256
    ) = AbiDecode::decode(raw.data.as_ref())?;

    Ok(SwapEvent {
        chain,
        pool: meta.pool,
        block_number: log.block_number.unwrap().as_u64(),
        tx_hash: log.transaction_hash.unwrap(),
        event_type: SwapType::BalancerSwap,

        amount0_in: Some(decoded.3),
        amount1_in: None,
        amount0_out: Some(decoded.4),
        amount1_out: None,

        reserve0: None,
        reserve1: None,
        tick: None,
        liquidity: None,
    })
}

pub async fn handle_swap(ev: SwapEvent) -> Result<()> {
    tracing::debug!(
        "[Balancer Swap] pool={} in={} out={}",
        ev.pool,
        ev.amount0_in.unwrap_or_default(),
        ev.amount0_out.unwrap_or_default()
    );

    // TODO: integrate with pricing engine
    Ok(())
}
