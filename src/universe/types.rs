use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct Protocol {
    pub name: String,
    pub category: String,
    pub chains: Vec<String>,
    pub tvl: f64,
}

#[derive(Debug, Deserialize)]
pub struct ChainTvl {
    pub name: String,
    pub tvl: f64,
}

#[derive(Debug, Deserialize)]
pub struct Stablecoin {
    pub symbol: String,
    pub circulating: f64,
    pub chains: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Universe {
    pub allowed_chains: Vec<String>,
    pub allowed_dexes: Vec<String>,
    pub allowed_tokens: Vec<String>,
}
