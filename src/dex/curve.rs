use anyhow::Result;
use ethers::abi::{AbiDecode, RawLog};
use ethers::types::{Log, U256};

use crate::types::{SwapEvent, SwapType, PairMeta};

/// Decode Curve TokenExchange event:
/// TokenExchange(address buyer, int128 sold_id, uint256 tokens_sold, int128 bought_id, uint256 tokens_bought)
pub fn decode_exchange(
    chain: String,
    meta: &PairMeta,
    log: &Log,
    raw: RawLog,
) -> Result<SwapEvent> {
    let decoded: (
        ethers::types::Address,
        i128,
        U256,
        i128,
        U256
    ) = AbiDecode::decode(raw.data.as_ref())?;

    Ok(SwapEvent {
        chain,
        pool: meta.pool,
        block_number: log.block_number.unwrap().as_u64(),
        tx_hash: log.transaction_hash.unwrap(),
        event_type: SwapType::CurveExchange,

        amount0_in: Some(decoded.2),
        amount1_in: None,
        amount0_out: Some(decoded.4),
        amount1_out: None,

        reserve0: None,
        reserve1: None,
        tick: None,
        liquidity: None,
    })
}

pub async fn handle_exchange(ev: SwapEvent) -> Result<()> {
    tracing::debug!(
        "[Curve Exchange] pool={} sold={} bought={}",
        ev.pool,
        ev.amount0_in.unwrap_or_default(),
        ev.amount0_out.unwrap_or_default()
    );

    // TODO: integrate with pricing engine
    Ok(())
}
