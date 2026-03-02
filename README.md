# Unified Multi-Chain Arbitrage Engine

This repository contains a full Rust + Solidity arbitrage engine with:

- Multi-chain support (Polygon, Ethereum, Arbitrum, Base)
- Multi-DEX ingestion (Uniswap V2/V3, Curve, Balancer, Algebra, Sushi, QuickSwap)
- Real-time event ingestion via HyperSync
- Dynamic pool discovery via HyperIndex
- Unified pricing engine
- Route planner + profit simulator
- Gas-aware arbitrage engine
- Solidity ArbExecutor with Foundry tests
- Deployment scripts (Rust + Foundry)
- Calldata builder module

See `docs/architecture.md` for a full overview.
