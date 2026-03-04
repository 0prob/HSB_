use anyhow::Result;
use arbitrage_engine::types::ChainConfig;
use arbitrage_engine::universe::filter::UniverseFilter;

fn load_chain(path: &str) -> Result<ChainConfig> {
    let raw = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&raw)?)
}

fn main() -> Result<()> {
    let chain = load_chain("config/polygon.toml")?;
    let _filter = UniverseFilter::from_chain(&chain)?;
    println!("Universe filter initialized for {}", chain.name);
    Ok(())
}
