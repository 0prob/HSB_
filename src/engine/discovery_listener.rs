use anyhow::Result;
use tokio::sync::broadcast::Receiver;

use crate::types::{ChainConfig, PairMeta};
use crate::engine::routing::RoutePlanner;
use crate::engine::subscriber::Subscriber;
use crate::engine::discovery::Discovery;

use crate::universe::events::UniverseEvent;
use crate::universe::manager::UniverseManager;

pub async fn run_discovery_listener(
    discovery: Discovery,
    subscriber: Subscriber,
    routing: RoutePlanner,
    universe_manager: UniverseManager,
    mut rx: Receiver<UniverseEvent>,
    chain: ChainConfig,
) -> Result<()> {
    loop {
        match rx.recv().await {
            Ok(UniverseEvent::Updated) => {
                let uni = universe_manager.get().await;

                if !uni.allowed_chains.iter().any(|c| c == &chain.name) {
                    tracing::warn!("Chain {} disabled by universe filter", chain.name);
                    continue;
                }

                // IMPORTANT: discovery must return a universe-filtered pool set
                let pools: Vec<PairMeta> = discovery.discover_pools(&chain.name, &uni).await?;

                // IMPORTANT: subscriber must subscribe ONLY from that same pool set
                subscriber.subscribe(&pools, &uni).await?;

                // IMPORTANT: routing graph must be built from the same pool set
                let graph = routing.build_token_graph_from_pools(&pools, &uni);
                routing.rebuild_routes(&graph);
                routing.rebuild_triangular_routes(&graph);

                tracing::info!(
                    "Universe update applied to chain {} pools={}",
                    chain.name,
                    pools.len()
                );
            }
            Err(_) => break,
        }
    }

    Ok(())
}
