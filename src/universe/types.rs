use ethers::types::Address;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct UniverseConfig {
    pub chain: String,

    // If non-empty: tokens must be in this set OR be base tokens
    pub allowed_tokens: HashSet<Address>,

    // Always excluded
    pub denied_tokens: HashSet<Address>,

    // Base tokens (routing anchors)
    pub base_tokens: HashSet<Address>,
    pub require_base_token: bool,

    // Optional DEX tag filtering (PairMeta.dex contains any of these substrings; case-insensitive)
    pub allowed_dex_substrings: Vec<String>,

    // Cap registry growth
    pub max_pools: usize,
}

impl UniverseConfig {
    pub fn empty(chain: &str) -> Self {
        Self {
            chain: chain.to_string(),
            allowed_tokens: HashSet::new(),
            denied_tokens: HashSet::new(),
            base_tokens: HashSet::new(),
            require_base_token: true,
            allowed_dex_substrings: vec![],
            max_pools: 50_000,
        }
    }
}
