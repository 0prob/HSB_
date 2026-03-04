use anyhow::Result;
use ethers::types::{Address, U256};

use crate::engine::routing::{Route, TriRoute};
use crate::executor::encode::encode_uniswap_v2_swap;
use crate::executor::types::EncodedRoute;

pub struct CalldataBuilder;

impl CalldataBuilder {
    /// Build a linear route (1-2 hops).
    /// NOTE: This is *not* valid production execution yet (pools are not routers).
    /// It is kept to make the crate compile and to support later executor integration.
    pub fn build_linear(
        &self,
        route: &Route,
        amount_in: U256,
        slippage_bps: u32,
        recipient: Address,
    ) -> Result<EncodedRoute> {
        let slip = U256::from(slippage_bps);
        let min_out = amount_in - (amount_in * slip / U256::from(10_000u64));

        let mut targets = Vec::new();
        let mut data = Vec::new();

        // tokens: [t0, t1, t2...]
        for (i, pool) in route.pools.iter().enumerate() {
            let token_in = route.tokens[i];
            let token_out = route.tokens[i + 1];

            let hop_data = encode_uniswap_v2_swap(
                amount_in,
                min_out,
                vec![token_in, token_out],
                recipient,
            )?;

            targets.push(*pool);
            data.push(hop_data);
        }

        Ok(EncodedRoute { targets, data })
    }

    /// Build a triangular route (A->B->C->A).
    pub fn build_triangular(
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
        data.push(encode_uniswap_v2_swap(amount_in, min_out, vec![tri.tokens[0], tri.tokens[1]], recipient)?);
        targets.push(tri.pools[0]);

        // B -> C
        data.push(encode_uniswap_v2_swap(min_out, min_out, vec![tri.tokens[1], tri.tokens[2]], recipient)?);
        targets.push(tri.pools[1]);

        // C -> A
        data.push(encode_uniswap_v2_swap(min_out, min_out, vec![tri.tokens[2], tri.tokens[0]], recipient)?);
        targets.push(tri.pools[2]);

        Ok(EncodedRoute { targets, data })
    }
}
