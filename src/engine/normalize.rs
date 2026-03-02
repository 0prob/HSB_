use anyhow::Result;

use crate::types::{SwapEvent, SwapType};
use crate::engine::pricing::PricingEngine;

/// Normalize swap events and forward them into the pricing engine.
/// This is the canonical entry point for all DEX events.
pub async fn process_event(
    pricing: &PricingEngine,
    ev: SwapEvent,
) -> Result<()> {
    match ev.event_type {
        SwapType::UniswapV2Swap => {
            pricing.update_v2_swap(ev).await?;
        }
        SwapType::UniswapV2Sync => {
            pricing.update_v2_sync(ev).await?;
        }
        SwapType::UniswapV3Swap => {
            pricing.update_v3_swap(ev).await?;
        }
        SwapType::CurveExchange => {
            pricing.update_curve(ev).await?;
        }
        SwapType::BalancerSwap => {
            pricing.update_balancer(ev).await?;
        }
        SwapType::AlgebraSwap => {
            pricing.update_algebra(ev).await?;
        }
        SwapType::SushiSwap | SwapType::QuickSwap => {
            // These are V2 forks — treat as V2 swaps
            pricing.update_v2_swap(ev).await?;
        }
    }

    Ok(())
}
