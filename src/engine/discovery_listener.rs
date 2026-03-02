use anyhow::Result;
use tokio::sync::broadcast::Receiver;

use crate::types::ChainConfig;
use crate::engine::registry::PairRegistry;
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

                if !uni.allowed_chains.contains(&chain.name) {
                    tracing::warn!("Chain {} disabled by universe filter", chain.name);
                    continue;
                }

                let pools = discovery.discover_pools(&chain.name, &uni).await?;
                subscriber.subscribe(&pools, &uni).await?;

                let graph = routing.build_token_graph(&uni);
                routing.rebuild_routes(&graph);
                routing.rebuild_triangular_routes(&graph);

                tracing::info!("Universe update applied to chain {}", chain.name);
            }
            Err(_) => break,
        }
    }

    Ok(())
}
