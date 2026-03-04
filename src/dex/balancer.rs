use anyhow::Result;
use ethers::abi::{AbiDecode, RawLog};
use ethers::types::{Log, U256};

use crate::types::{PairMeta, SwapEvent, SwapType};

pub fn decode_swap(chain: String, meta: &PairMeta, log: &Log, raw: RawLog) -> Result<SwapEvent> {
    let (_pool_id, _token_in, _token_out, amount_in, amount_out): (ethers::types::H256, ethers::types::Address, ethers::types::Address, U256, U256) =
        AbiDecode::decode(raw.data)?;

    Ok(SwapEvent {
        chain,
        pool: meta.pool,
        block_number: log.block_number.unwrap().as_u64(),
        tx_hash: log.transaction_hash.unwrap(),
        event_type: SwapType::BalancerSwap,
        amount0_in: Some(amount_in),
        amount0_out: Some(amount_out),
        amount1_in: None,
        amount1_out: None,
        reserve0: None,
        reserve1: None,
        tick: None,
        liquidity: None,
    })
}
