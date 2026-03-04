use anyhow::Result;
use ethers::abi::{AbiDecode, RawLog};
use ethers::types::{I256, Log, U256};

use crate::engine::pricing::PricingEngine;
use crate::types::{PairMeta, SwapEvent, SwapType};

fn u256_from_i256_abs(x: I256) -> U256 {
    // Convert |x| to U256 using big-endian bytes
    let ux: U256 = if x >= I256::zero() {
        x.into_raw()
    } else {
        (-x).into_raw()
    };

    let mut buf = [0u8; 32];
    ux.to_big_endian(&mut buf);
    U256::from_big_endian(&buf)
}

/// Algebra Swap (UniV3-style):
/// event Swap(address indexed sender, address indexed recipient,
///            int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick)
///
/// Indexed fields (sender, recipient) are in topics; data holds:
/// (amount0, amount1, sqrtPriceX96, liquidity, tick)
pub fn decode_swap(chain: String, meta: &PairMeta, log: &Log, raw: RawLog) -> Result<SwapEvent> {
    let (amount0, amount1, _sqrt_price_x96, liquidity, tick): (I256, I256, U256, U256, i32) =
        AbiDecode::decode(raw.data)?;

    let (amount0_in, amount0_out) = if amount0 >= I256::zero() {
        (Some(u256_from_i256_abs(amount0)), None)
    } else {
        (None, Some(u256_from_i256_abs(amount0)))
    };

    let (amount1_in, amount1_out) = if amount1 >= I256::zero() {
        (Some(u256_from_i256_abs(amount1)), None)
    } else {
        (None, Some(u256_from_i256_abs(amount1)))
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
        tick: Some(tick),
        liquidity: Some(liquidity),
    })
}

pub async fn handle_swap(ev: SwapEvent, pricing: &PricingEngine) -> Result<()> {
    pricing.update_algebra(ev).await
}
