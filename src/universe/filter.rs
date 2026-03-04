use anyhow::{anyhow, Result};
use ethers::types::Address;
use std::collections::HashSet;

use crate::types::{ChainConfig, PairMeta};
use crate::universe::types::UniverseConfig;

fn parse_addr_list(s: &str) -> Result<HashSet<Address>> {
    let mut out = HashSet::new();
    for part in s.split(',') {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        let a: Address = t.parse().map_err(|e| anyhow!("bad address '{}': {:?}", t, e))?;
        out.insert(a);
    }
    Ok(out)
}

fn env_key(prefix: &str, chain: &str) -> String {
    format!("{}_{}", prefix, chain.to_uppercase())
}

fn default_polygon_base_tokens() -> HashSet<Address> {
    let mut s = HashSet::new();
    s.insert("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".parse().unwrap()); // USDC
    s.insert("0xC2132D05D31c914a87C6611C10748AEb04B58e8F".parse().unwrap()); // USDT
    s.insert("0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063".parse().unwrap()); // DAI
    s.insert("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".parse().unwrap()); // WMATIC
    s.insert("0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619".parse().unwrap()); // WETH
    s
}

fn default_allowed_dex_substrings_polygon() -> Vec<String> {
    vec!["quickswap_v2".to_string(), "quickswap_algebra".to_string()]
}

#[derive(Clone)]
pub struct UniverseFilter {
    cfg: UniverseConfig,
}

impl UniverseFilter {
    pub fn new(cfg: UniverseConfig) -> Self {
        Self { cfg }
    }

    pub fn from_chain(chain: &ChainConfig) -> Result<Self> {
        let mut cfg = UniverseConfig::empty(&chain.name);

        // Defaults per chain
        if chain.chain_id == 137 || chain.name.to_lowercase() == "polygon" {
            cfg.base_tokens = default_polygon_base_tokens();
            cfg.allowed_dex_substrings = default_allowed_dex_substrings_polygon();
            cfg.require_base_token = true;
            cfg.max_pools = 50_000;
        }

        let chain_key = chain.name.as_str();

        if let Ok(v) = std::env::var(env_key("UNIVERSE_ALLOWED_TOKENS", chain_key)) {
            cfg.allowed_tokens = parse_addr_list(&v)?;
        }
        if let Ok(v) = std::env::var(env_key("UNIVERSE_DENIED_TOKENS", chain_key)) {
            cfg.denied_tokens = parse_addr_list(&v)?;
        }
        if let Ok(v) = std::env::var(env_key("UNIVERSE_BASE_TOKENS", chain_key)) {
            cfg.base_tokens = parse_addr_list(&v)?;
        }
        if let Ok(v) = std::env::var(env_key("UNIVERSE_REQUIRE_BASE_TOKEN", chain_key)) {
            cfg.require_base_token = v.trim().eq_ignore_ascii_case("true");
        }
        if let Ok(v) = std::env::var(env_key("UNIVERSE_ALLOWED_DEX", chain_key)) {
            cfg.allowed_dex_substrings = v
                .split(',')
                .map(|x| x.trim().to_lowercase())
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>();
        }
        if let Ok(v) = std::env::var(env_key("UNIVERSE_MAX_POOLS", chain_key)) {
            if let Ok(n) = v.trim().parse::<usize>() {
                cfg.max_pools = n;
            }
        }

        Ok(Self::new(cfg))
    }

    pub fn max_pools(&self) -> usize {
        self.cfg.max_pools
    }

    pub fn accept_pair(&self, meta: &PairMeta) -> bool {
        let t0 = meta.token0;
        let t1 = meta.token1;

        if self.cfg.denied_tokens.contains(&t0) || self.cfg.denied_tokens.contains(&t1) {
            return false;
        }

        if !self.cfg.allowed_dex_substrings.is_empty() {
            let dex = meta.dex.to_lowercase();
            let ok = self
                .cfg
                .allowed_dex_substrings
                .iter()
                .any(|s| dex.contains(s));
            if !ok {
                return false;
            }
        }

        if self.cfg.require_base_token {
            let has_base = self.cfg.base_tokens.contains(&t0) || self.cfg.base_tokens.contains(&t1);
            if !has_base {
                return false;
            }
        }

        if !self.cfg.allowed_tokens.is_empty() {
            let ok0 = self.cfg.allowed_tokens.contains(&t0) || self.cfg.base_tokens.contains(&t0);
            let ok1 = self.cfg.allowed_tokens.contains(&t1) || self.cfg.base_tokens.contains(&t1);
            if !(ok0 && ok1) {
                return false;
            }
        }

        true
    }

    pub fn accept_route_tokens(&self, tokens: &[Address]) -> bool {
        if tokens.is_empty() {
            return false;
        }
        for t in tokens {
            if self.cfg.denied_tokens.contains(t) {
                return false;
            }
            if !self.cfg.allowed_tokens.is_empty() {
                if !(self.cfg.allowed_tokens.contains(t) || self.cfg.base_tokens.contains(t)) {
                    return false;
                }
            }
        }
        true
    }
}
