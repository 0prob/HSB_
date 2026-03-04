use anyhow::Result;
use ethers::abi::RawLog;
use ethers::types::{Bytes, H160, H256, Log as EthersLog, U64};

use hypersync_client::simple_types::Log as HsLog;

use crate::engine::normalize::normalize_and_update;
use crate::engine::pricing::PricingEngine;
use crate::engine::registry::PairRegistry;

fn topic(sig: &str) -> H256 {
    H256::from(ethers::utils::keccak256(sig.as_bytes()))
}

fn fixed20_to_h160(x: &impl AsRef<[u8]>) -> H160 {
    H160::from_slice(x.as_ref())
}

fn fixed32_to_h256(x: &impl AsRef<[u8]>) -> H256 {
    H256::from_slice(x.as_ref())
}

fn hs_to_ethers_log(lg: &HsLog) -> Option<EthersLog> {
    let addr = lg.address.as_ref()?;
    let txh = lg.transaction_hash.as_ref()?;
    let bn_u = lg.block_number.as_ref()?; // UInt
    let data = lg.data.as_ref()?;

    // UInt implements Into<u64> in hypersync-client simple types
    let bn: u64 = u64::from(*bn_u);

    let topics: Vec<H256> = lg
        .topics
        .iter()
        .filter_map(|t| t.as_ref())
        .map(|t| fixed32_to_h256(t))
        .collect();

    Some(EthersLog {
        address: fixed20_to_h160(addr),
        topics,
        data: Bytes::from(data.as_ref().to_vec()),
        block_number: Some(U64::from(bn)),
        transaction_hash: Some(fixed32_to_h256(txh)),
        ..Default::default()
    })
}

pub async fn decode_log(
    chain: String,
    log: HsLog,
    registry: PairRegistry,
    pricing: PricingEngine,
) -> Result<()> {
    let log = match hs_to_ethers_log(&log) {
        Some(l) => l,
        None => return Ok(()),
    };

    let topic0 = match log.topics.get(0) {
        Some(t) => *t,
        None => return Ok(()),
    };

    let pool = log.address;
    let meta = match registry.get(&pool) {
        Some(m) => m,
        None => return Ok(()),
    };

    let raw = RawLog {
        topics: log.topics.clone(),
        data: log.data.to_vec(),
    };

    // Uniswap V2 Swap
    if topic0 == topic("Swap(address,uint256,uint256,uint256,uint256,address)") {
        let ev = crate::dex::uniswap_v2::decode_swap(chain, &meta, &log, raw)?;
        return normalize_and_update(&pricing, ev).await;
    }

    // Uniswap V2 Sync (reserve updates)
    if topic0 == topic("Sync(uint112,uint112)") {
        let ev = crate::dex::uniswap_v2::decode_sync(chain, &meta, &log, raw)?;
        return normalize_and_update(&pricing, ev).await;
    }

    // Algebra Swap (UniV3-style signature)
    if topic0 == topic("Swap(address,address,int256,int256,uint160,uint128,int24)") {
        // only handle if this pool was tagged as algebra
        if meta.dex.to_lowercase().contains("algebra") {
            let ev = crate::dex::algebra::decode_swap(chain, &meta, &log, raw)?;
            return normalize_and_update(&pricing, ev).await;
        }
    }

    Ok(())
}
