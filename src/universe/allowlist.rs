use anyhow::Result;
use ethers::types::Address;
use serde::{Deserialize, Serialize};

use crate::types::ChainConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniverseAllowlist {
    pub chain: String,
    pub allowed_tokens: Vec<String>,
}

impl UniverseAllowlist {
    pub fn from_chain(chain: &ChainConfig) -> Result<Self> {
        // Convert any configured addresses into strings.
        // If your ChainConfig doesn’t include allowlist tokens yet, this remains empty.
        let mut allowed_tokens: Vec<String> = Vec::new();

        // Optional: read from ENV var to keep moving without changing config schema.
        // Comma-separated 0x addresses.
        let env_key = format!("UNIVERSE_ALLOWLIST_{}", chain.name.to_uppercase());
        if let Ok(val) = std::env::var(env_key) {
            for part in val.split(',') {
                let s = part.trim();
                if s.is_empty() {
                    continue;
                }
                // Validate formatting
                let _addr: Address = s.parse()?;
                allowed_tokens.push(s.to_string());
            }
        }

        Ok(Self {
            chain: chain.name.clone(),
            allowed_tokens,
        })
    }
}
