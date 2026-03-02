use anyhow::Result;
use reqwest::Client;

use super::types::{Protocol, ChainTvl, Stablecoin};

const BASE: &str = "https://pro-api.llama.fi";

pub struct DefiLlama {
    client: Client,
}

impl DefiLlama {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn protocols(&self) -> Result<Vec<Protocol>> {
        let url = format!("{}/api/protocols", BASE);
        let resp = self.client.get(url).send().await?.json().await?;
        Ok(resp)
    }

    pub async fn chains(&self) -> Result<Vec<ChainTvl>> {
        let url = format!("{}/api/v2/chains", BASE);
        let resp = self.client.get(url).send().await?.json().await?;
        Ok(resp)
    }

    pub async fn stablecoins(&self) -> Result<Vec<Stablecoin>> {
        let url = format!("{}/stablecoins/stablecoins", BASE);
        let resp = self.client.get(url).send().await?.json().await?;
        Ok(resp)
    }
}
