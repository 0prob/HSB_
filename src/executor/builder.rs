use anyhow::Result;
use ethers::types::{Address, Bytes, U256};

use crate::engine::routing::Route;
use crate::executor::types::{Hop, EncodedRoute};
use crate::executor::encode::*;

pub struct CalldataBuilder;

impl CalldataBuilder {
    pub fn build(
        &self,
        route: &Route,
        amount_in: U256,
        slippage_bps: u32,
    ) -> Result<EncodedRoute> {
        let mut targets = Vec::new();
        let mut data = Vec::new();

        let slip = U256::from(slippage_bps);
        let min_out = amount_in - (amount_in * slip / U256::from(10_000u64));

        for pool in &route.pools {
            // TODO: detect DEX type from registry metadata
            // For now, assume Uniswap V2-style
            let hop_data = encode_uniswap_v2_swap(
                amount_in,
                min_out,
                vec![Address::zero(), Address::zero()],
                Address::zero(),
            )?;

            targets.push(*pool);
            data.push(hop_data);
        }

        Ok(EncodedRoute { targets, data })
    }
}
