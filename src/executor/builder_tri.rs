use anyhow::Result;
use ethers::types::{Address, U256};

use crate::engine::routing::TriRoute;
use crate::executor::encode::encode_uniswap_v2_swap;
use crate::executor::types::EncodedRoute;

/// Triangular builder assumes V2-style pools for now (A->B->C->A).
pub struct TriCalldataBuilder;

impl TriCalldataBuilder {
    pub fn build(
        &self,
        tri: &TriRoute,
        amount_in: U256,
        slippage_bps: u32,
        recipient: Address,
    ) -> Result<EncodedRoute> {
        let slip = U256::from(slippage_bps);
        let min_out = amount_in - (amount_in * slip / U256::from(10_000u64));

        let mut targets = Vec::new();
        let mut data = Vec::new();

        // A -> B
        let hop1 = encode_uniswap_v2_swap(amount_in, min_out, vec![tri.tokens[0], tri.tokens[1]], recipient)?;
        targets.push(tri.pools[0]);
        data.push(hop1);

        // B -> C
        let hop2 = encode_uniswap_v2_swap(min_out, min_out, vec![tri.tokens[1], tri.tokens[2]], recipient)?;
        targets.push(tri.pools[1]);
        data.push(hop2);

        // C -> A
        let hop3 = encode_uniswap_v2_swap(min_out, min_out, vec![tri.tokens[2], tri.tokens[0]], recipient)?;
        targets.push(tri.pools[2]);
        data.push(hop3);

        Ok(EncodedRoute { targets, data })
    }
}
