use anyhow::Result;

use crate::executor::types::EncodedRoute;
use crate::types::ChainConfig;

/// Bundle submission is intentionally stubbed in this build.
///
/// Reason:
/// - Your current ExecutionConfig does not include relay fields
/// - The engine does not yet build valid on-chain calldata for router/vault execution
/// - You should switch to a real executor contract + signed tx flow or a relay config later
pub async fn submit_bundle(_chain: &ChainConfig, _encoded: EncodedRoute, _profit_usd: f64) -> Result<()> {
    Ok(())
}
