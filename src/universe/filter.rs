use anyhow::Result;

use super::fetch::DefiLlama;
use super::types::Universe;

pub struct UniverseFilter {
    llama: DefiLlama,
}

impl UniverseFilter {
    pub fn new() -> Self {
        Self {
            llama: DefiLlama::new(),
        }
    }

    pub async fn build(&self) -> Result<Universe> {
        let protocols = self.llama.protocols().await?;
        let chains = self.llama.chains().await?;
        let stablecoins = self.llama.stablecoins().await?;

        let allowed_chains: Vec<String> = chains
            .into_iter()
            .filter(|c| c.tvl > 1_000_000_000.0)
            .map(|c| c.name)
            .collect();

        let allowed_dexes: Vec<String> = protocols
            .iter()
            .filter(|p| p.category.to_lowercase().contains("dex"))
            .filter(|p| p.tvl > 100_000_000.0)
            .map(|p| p.name.clone())
            .collect();

        let allowed_tokens: Vec<String> = stablecoins
            .into_iter()
            .filter(|s| s.circulating > 500_000_000.0)
            .map(|s| s.symbol)
            .collect();

        Ok(Universe {
            allowed_chains,
            allowed_dexes,
            allowed_tokens,
        })
    }
}
