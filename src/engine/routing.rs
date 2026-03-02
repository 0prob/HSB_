use anyhow::Result;
use ethers::types::Address;

use crate::engine::registry::PairRegistry;
use crate::engine::pricing::PricingEngine;

/// A route is a sequence of pools to traverse.
#[derive(Debug, Clone)]
pub struct Route {
    pub pools: Vec<Address>,
    pub price: f64,
}

/// A triangular route: token A -> B -> C -> A
#[derive(Debug, Clone)]
pub struct TriRoute {
    pub pools: [Address; 3],
    pub composite_price: f64,
}

pub struct RoutePlanner {
    registry: PairRegistry,
    pricing: PricingEngine,
}

impl RoutePlanner {
    pub fn new(registry: PairRegistry, pricing: PricingEngine) -> Self {
        Self { registry, pricing }
    }
    let graph = routing.build_token_graph(&uni);
    let routes = routing.build_routes_from_graph(&graph);
    let tri_routes = routing.build_triangular_routes(&graph);
    /// Build all 1‑hop and 2‑hop routes for a given chain.
    pub fn build_routes(&self, chain: &str, max_hops: usize) -> Vec<Route> {
        let pools = self.registry.by_chain(chain);
        let mut routes = Vec::new();

        // 1‑hop
        for p in &pools {
            if let Some(price) = self.pricing.get_price(&p.pool) {
                routes.push(Route {
                    pools: vec![p.pool],
                    price: price.price,
                });
            }
        }

        if max_hops < 2 {
            return routes;
        }

        // 2‑hop
        for a in &pools {
            for b in &pools {
                if a.pool == b.pool {
                    continue;
                }

                let pa = self.pricing.get_price(&a.pool);
                let pb = self.pricing.get_price(&b.pool);

                if let (Some(pa), Some(pb)) = (pa, pb) {
                    let combined = pa.price * pb.price;
                    routes.push(Route {
                        pools: vec![a.pool, b.pool],
                        price: combined,
                    });
                }
            }
        }

        routes
    }

    /// Build triangular (3‑hop) routes A->B->C->A.
    /// This is intentionally naive: it just looks for 3 pools that form a cycle.
    pub fn build_triangular_routes(&self, chain: &str) -> Vec<TriRoute> {
        let pools = self.registry.by_chain(chain);
        let mut out = Vec::new();

        // Very simple: any 3 distinct pools with prices -> composite price = p1 * p2 * p3
        for i in 0..pools.len() {
            for j in 0..pools.len() {
                for k in 0..pools.len() {
                    if i == j || j == k || i == k {
                        continue;
                    }

                    let pi = match self.pricing.get_price(&pools[i].pool) {
                        Some(p) => p.price,
                        None => continue,
                    };
                    let pj = match self.pricing.get_price(&pools[j].pool) {
                        Some(p) => p.price,
                        None => continue,
                    };
                    let pk = match self.pricing.get_price(&pools[k].pool) {
                        Some(p) => p.price,
                        None => continue,
                    };

                    let composite = pi * pj * pk;

                    out.push(TriRoute {
                        pools: [pools[i].pool, pools[j].pool, pools[k].pool],
                        composite_price: composite,
                    });
                }
            }
        }

        out
    }
}
