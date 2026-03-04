use anyhow::Result;
use ethers::abi::{AbiDecode, RawLog};
use ethers::types::{Log, U256};

use crate::types::{PairMeta, SwapEvent, SwapType};

pub fn decode_exchange(chain: String, meta: &PairMeta, log: &Log, raw: RawLog) -> Result<SwapEvent> {
    let (_buyer, _sold_id, tokens_sold, _bought_id, tokens_bought): (ethers::types::Address, i128, U256, i128, U256) =
        AbiDecode::decode(raw.data)?;

    Ok(SwapEvent {
        chain,
        pool: meta.pool,
        block_number: log.block_number.unwrap().as_u64(),
        tx_hash: log.transaction_hash.unwrap(),
        event_type: SwapType::CurveExchange,
        amount0_in: Some(tokens_sold),
        amount0_out: Some(tokens_bought),
        amount1_in: None,
        amount1_out: None,
        reserve0: None,
        reserve1: None,
        tick: None,
        liquidity: None,
    })
}
