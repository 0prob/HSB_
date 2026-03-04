use anyhow::{anyhow, Result};
use ethers::contract::abigen;
use ethers::providers::{Http, Middleware, Provider};
use ethers::types::{Address, BlockId, BlockNumber, U64};
use std::sync::Arc;
use std::time::Duration;

abigen!(
    UniswapV2Pair,
    r#"[
        function getReserves() view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
    ]"#,
);

#[derive(Clone)]
pub struct RpcSnapshot {
    provider: Arc<Provider<Http>>,
}

impl RpcSnapshot {
    pub fn new(rpc_url: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| anyhow!("bad rpc_url {}: {:?}", rpc_url, e))?
            .interval(Duration::from_millis(200));
        Ok(Self {
            provider: Arc::new(provider),
        })
    }

    pub async fn latest_block_number(&self) -> Result<u64> {
        let bn: U64 = self.provider.get_block_number().await?;
        Ok(bn.as_u64())
    }

    pub async fn v2_get_reserves_at(&self, pair: Address, block: u64) -> Result<(u128, u128)> {
        let c = UniswapV2Pair::new(pair, self.provider.clone());

        let bid = BlockId::Number(BlockNumber::Number(U64::from(block)));
        let (r0, r1, _ts) = c.get_reserves().block(bid).call().await?;

        Ok((r0 as u128, r1 as u128))
    }
}
