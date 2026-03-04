use anyhow::Result;

use crate::engine::pricing::PricingEngine;
use crate::types::{SwapEvent, SwapType};

pub async fn normalize_and_update(pricing: &PricingEngine, ev: SwapEvent) -> Result<()> {
    match ev.event_type {
        SwapType::UniswapV2Swap => pricing.update_v2_swap(ev).await?,
        SwapType::UniswapV2Sync => pricing.update_v2_sync(ev).await?,
        SwapType::UniswapV3Swap => pricing.update_v3_swap(ev).await?,
        SwapType::CurveExchange => pricing.update_curve(ev).await?,
        SwapType::BalancerSwap => pricing.update_balancer(ev).await?,
        SwapType::AlgebraSwap => pricing.update_algebra(ev).await?,
        _ => {}
    }
    Ok(())
}
