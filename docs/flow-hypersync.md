# HyperSync Flow

1. Build dynamic filter from PairRegistry.
2. Subscribe to logs for all pools.
3. Decode logs into SwapEvent.
4. Forward SwapEvent into normalization layer.
5. Update pricing engine.

HyperSync provides:
- Low-latency log streaming
- Efficient multi-address filtering
- Stable ordering guarantees
