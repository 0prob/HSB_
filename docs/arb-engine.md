# ArbEngine

The ArbEngine is responsible for:

1. Building routes via RoutePlanner.
2. Simulating profitability via ProfitSimulator.
3. Selecting the best profitable route.
4. Emitting actionable opportunities.
5. (Optional) Calling ArbExecutor with encoded calldata.

The engine runs continuously and evaluates opportunities every 500ms.
