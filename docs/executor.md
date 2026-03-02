# ArbExecutor

The ArbExecutor is a minimal Solidity contract that:

- Executes multi-hop calldata bundles
- Enforces slippage via minReturn
- Returns profit to owner
- Supports arbitrary DEX calls
- Is chain-agnostic and DEX-agnostic

The Rust calldata builder compiles routes into ABI-encoded calls.
