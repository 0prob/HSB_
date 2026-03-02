use anyhow::Result;
use arbitrage_engine::universe::filter::UniverseFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let filter = UniverseFilter::new();
    let uni = filter.build().await?;

    std::fs::create_dir_all("universe")?;
    std::fs::write(
        "universe/allowed_chains.json",
        serde_json::to_string_pretty(&uni.allowed_chains)?,
    )?;
    std::fs::write(
        "universe/allowed_dexes.json",
        serde_json::to_string_pretty(&uni.allowed_dexes)?,
    )?;
    std::fs::write(
        "universe/allowed_tokens.json",
        serde_json::to_string_pretty(&uni.allowed_tokens)?,
    )?;

    println!("Universe built.");
    Ok(())
}
