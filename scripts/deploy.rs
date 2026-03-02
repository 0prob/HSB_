use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use std::env;

abigen!(
    ArbExecutor,
    "executor/out/ArbExecutor.sol/ArbExecutor.json"
);

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let rpc = env::var("RPC_URL")?;
    let pk = env::var("DEPLOYER_PRIVATE_KEY")?;

    let wallet: LocalWallet = pk.parse()?;
    let provider = Provider::<Http>::try_from(rpc)?;
    let client = SignerMiddleware::new(provider, wallet.with_chain_id(137u64));
    let client = Arc::new(client);

    let factory = ArbExecutor::deploy(client.clone(), ())?;
    let contract = factory.send().await?;

    println!("ArbExecutor deployed at: {}", contract.address());
    Ok(())
}
