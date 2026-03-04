use anyhow::Result;
use ethers::abi::{AbiDecode, RawLog};
use ethers::types::{I256, Log, U256};

use crate::types::{PairMeta, SwapEvent, SwapType};

pub fn decode_swap(chain: String, meta: &PairMeta, log: &Log, raw: RawLog) -> Result<SwapEvent> {
    let (amount0, amount1, _sqrt_price_x96, liquidity, tick): (I256, I256, U256, U256, i32) =
        AbiDecode::decode(raw.data)?;

    let (amount0_in, amount0_out) = if amount0 >= I256::zero() {
        (Some(amount0.into_raw()), None)
    } else {
        (None, Some((-amount0).into_raw()))
    };

    let (amount1_in, amount1_out) = if amount1 >= I256::zero() {
        (Some(amount1.into_raw()), None)
    } else {
        (None, Some((-amount1).into_raw()))
    };

    Ok(SwapEvent {
        chain,
        pool: meta.pool,
        block_number: log.block_number.unwrap().as_u64(),
        tx_hash: log.transaction_hash.unwrap(),
        event_type: SwapType::UniswapV3Swap,
        amount0_in: amount0_in.map(U256::from),
        amount1_in: amount1_in.map(U256::from),
        amount0_out: amount0_out.map(U256::from),
        amount1_out: amount1_out.map(U256::from),
        reserve0: None,
        reserve1: None,
        tick: Some(tick),
        liquidity: Some(liquidity),
    })
}
