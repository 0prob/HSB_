use anyhow::Result;
use ethers::abi::{AbiDecode, RawLog};
use ethers::types::{Log, U256};

use crate::engine::pricing::PricingEngine;
use crate::types::{PairMeta, SwapEvent, SwapType};

pub fn decode_swap(
    chain_id: u64,
    chain: String,
    meta: &PairMeta,
    log: &Log,
    raw: RawLog,
) -> Result<SwapEvent> {
    let (amount0_in, amount1_in, amount0_out, amount1_out): (U256, U256, U256, U256) =
        AbiDecode::decode(raw.data)?;

    Ok(SwapEvent {
        chain_id,
        chain,
        pool: meta.pool,
        block_number: log.block_number.unwrap().as_u64(),
        tx_hash: log.transaction_hash.unwrap(),
        event_type: SwapType::UniswapV2Swap,

        amount0_in: Some(amount0_in),
        amount1_in: Some(amount1_in),
        amount0_out: Some(amount0_out),
        amount1_out: Some(amount1_out),

        reserve0: None,
        reserve1: None,
        tick: None,
        liquidity: None,
    })
}

pub fn decode_sync(
    chain_id: u64,
    chain: String,
    meta: &PairMeta,
    log: &Log,
    raw: RawLog,
) -> Result<SwapEvent> {
    let (reserve0, reserve1): (U256, U256) = AbiDecode::decode(raw.data)?;

    Ok(SwapEvent {
        chain_id,
        chain,
        pool: meta.pool,
        block_number: log.block_number.unwrap().as_u64(),
        tx_hash: log.transaction_hash.unwrap(),
        event_type: SwapType::UniswapV2Sync,

        amount0_in: None,
        amount1_in: None,
        amount0_out: None,
        amount1_out: None,

        reserve0: Some(reserve0),
        reserve1: Some(reserve1),
        tick: None,
        liquidity: None,
    })
}

pub async fn handle_swap(_ev: SwapEvent, _pricing: &PricingEngine) -> Result<()> {
    // IMPORTANT: ignore Swap events for V2 pricing (unstable without direction/decimals).
    Ok(())
}

pub async fn handle_sync(ev: SwapEvent, pricing: &PricingEngine) -> Result<()> {
    pricing.update_v2_sync(ev).await
}
