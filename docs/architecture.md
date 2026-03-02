# Architecture Overview

## Components

### 1. HyperIndex Discovery
Discovers pools across all supported DEXes and chains.
Outputs `PairMeta` into the PairRegistry.

### 2. HyperSync Subscriber
Subscribes to real-time logs for all pools.
Decodes events into normalized `SwapEvent` structs.

### 3. Pricing Engine
Maintains per-pool price state using:
- V2 reserves
- V3 ticks
- Curve spot prices
- Balancer weighted math
- Algebra tick math

### 4. Route Planner
Builds 1-hop and 2-hop arbitrage routes.

### 5. Profit Simulator
Computes expected output, slippage, and gas-adjusted profit.

### 6. ArbEngine
Evaluates all routes and selects profitable opportunities.

### 7. ArbExecutor (Solidity)
Executes calldata bundles on-chain.

### 8. Calldata Builder
Compiles routes into ABI-encoded calldata for ArbExecutor.

See the other docs for subsystem details.
