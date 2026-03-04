use anyhow::{anyhow, Result};
use dashmap::DashMap;
use ethers::contract::abigen;
use ethers::providers::{Http, Provider};
use ethers::types::Address;
use std::sync::Arc;
use std::time::Duration;

abigen!(
    Erc20Decimals,
    r#"[
        function decimals() view returns (uint8)
    ]"#,
);

#[derive(Clone)]
pub struct DecimalsCache {
    provider: Arc<Provider<Http>>,
    cache: Arc<DashMap<Address, u8>>,
}

impl DecimalsCache {
    pub fn new(rpc_url: &str) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)
            .map_err(|e| anyhow!("bad rpc_url {}: {:?}", rpc_url, e))?
            .interval(Duration::from_millis(250));

        Ok(Self {
            provider: Arc::new(provider),
            cache: Arc::new(DashMap::new()),
        })
    }

    pub fn get_cached(&self, token: Address) -> Option<u8> {
        self.cache.get(&token).map(|v| *v)
    }

    pub async fn get_or_fetch(&self, token: Address) -> Result<u8> {
        if let Some(d) = self.get_cached(token) {
            return Ok(d);
        }

        let c = Erc20Decimals::new(token, self.provider.clone());
        let d = c.decimals().call().await.unwrap_or(18u8);

        self.cache.insert(token, d);
        Ok(d)
    }

    pub async fn preload<I: IntoIterator<Item = Address>>(&self, toks: I) {
        for t in toks {
            let _ = self.get_or_fetch(t).await;
        }
    }
}
