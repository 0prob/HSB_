use ethers::types::Address;
use std::collections::{HashMap, HashSet};

use crate::engine::pricing::PricingEngine;
use crate::engine::registry::PairRegistry;

#[derive(Debug, Clone)]
pub struct Route {
    pub pools: Vec<Address>,   // length 1 or 2
    pub tokens: Vec<Address>,  // [A,B] or [A,B,A]
    pub price: f64,            // heuristic multiplier
}

#[derive(Debug, Clone)]
pub struct TriRoute {
    pub pools: [Address; 3],
    pub tokens: [Address; 4], // [A,B,C,A]
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

    fn pool_dir_price(&self, pool: Address, token_in: Address, token_out: Address) -> Option<f64> {
        let meta = self.registry.get(&pool)?;
        let p = self.pricing.get_price(&pool)?.price;
        if p <= 0.0 {
            return None;
        }

        if token_in == meta.token0 && token_out == meta.token1 {
            Some(p)
        } else if token_in == meta.token1 && token_out == meta.token0 {
            Some(1.0 / p)
        } else {
            None
        }
    }

    // 1-hop "route" (not an arbitrage cycle; kept for prompt completeness / diagnostics)
    pub fn build_1hop_routes(&self, chain: &str) -> Vec<Route> {
        let metas = self.registry.by_chain(chain);
        let mut out = Vec::new();
        for p in metas {
            if self.pricing.get_price(&p.pool).is_none() {
                continue;
            }
            // Two directions (A->B and B->A)
            for (a,b) in [(p.token0,p.token1),(p.token1,p.token0)] {
                let pr = match self.pool_dir_price(p.pool, a, b) {
                    Some(x) => x,
                    None => continue,
                };
                out.push(Route { pools: vec![p.pool], tokens: vec![a,b], price: pr });
            }
        }
        out
    }

    // 2-hop cycles only: A -> B -> A
    pub fn build_2hop_cycles(&self, chain: &str) -> Vec<Route> {
        let metas = self.registry.by_chain(chain);
        let mut out = Vec::new();

        for p1 in &metas {
            for p2 in &metas {
                if p1.pool == p2.pool {
                    continue;
                }
                if self.pricing.get_price(&p1.pool).is_none() || self.pricing.get_price(&p2.pool).is_none() {
                    continue;
                }

                for (a,b) in [(p1.token0,p1.token1),(p1.token1,p1.token0)] {
                    let p_ab = match self.pool_dir_price(p1.pool, a, b) { Some(x) => x, None => continue };

                    // Require p2 supports B->A
                    let p_ba = match self.pool_dir_price(p2.pool, b, a) { Some(x) => x, None => continue };

                    out.push(Route {
                        pools: vec![p1.pool, p2.pool],
                        tokens: vec![a, b, a],
                        price: p_ab * p_ba,
                    });
                }
            }
        }

        out
    }

    // Triangular cycles: A -> B -> C -> A using token graph from PairMeta
    pub fn build_triangular_cycles(&self, chain: &str) -> Vec<TriRoute> {
        let metas = self.registry.by_chain(chain);

        // adjacency token -> list of (next_token, pool)
        let mut adj: HashMap<Address, Vec<(Address, Address)>> = HashMap::new();
        for p in &metas {
            if self.pricing.get_price(&p.pool).is_none() {
                continue;
            }
            adj.entry(p.token0).or_default().push((p.token1, p.pool));
            adj.entry(p.token1).or_default().push((p.token0, p.pool));
        }

        let mut out = Vec::new();
        let mut seen: HashSet<(Address, Address, Address)> = HashSet::new();

        for (&a, edges_ab) in &adj {
            for &(b, p1) in edges_ab {
                let p_ab = match self.pool_dir_price(p1, a, b) { Some(x) => x, None => continue };

                let Some(edges_bc) = adj.get(&b) else { continue; };
                for &(c, p2) in edges_bc {
                    if c == a { continue; }
                    if p2 == p1 { continue; }
                    let p_bc = match self.pool_dir_price(p2, b, c) { Some(x) => x, None => continue };

                    let Some(edges_ca) = adj.get(&c) else { continue; };
                    // find edge back to a
                    for &(a2, p3) in edges_ca {
                        if a2 != a { continue; }
                        if p3 == p1 || p3 == p2 { continue; }

                        // de-duplicate triangles by ordered key
                        let key = (a, b, c);
                        if !seen.insert(key) { continue; }

                        let p_ca = match self.pool_dir_price(p3, c, a) { Some(x) => x, None => continue };

                        out.push(TriRoute {
                            pools: [p1, p2, p3],
                            tokens: [a, b, c, a],
                            composite_price: p_ab * p_bc * p_ca,
                        });
                    }
                }
            }
        }

        out
    }
}
