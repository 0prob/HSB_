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
    let pk = env::var("CALLER_PRIVATE_KEY")?;
    let exec_addr: Address = env::var("EXECUTOR_ADDRESS")?.parse()?;

    let wallet: LocalWallet = pk.parse()?;
    let provider = Provider::<Http>::try_from(rpc)?;
    let client = SignerMiddleware::new(provider, wallet.with_chain_id(137u64));
    let client = Arc::new(client);

    let exec = ArbExecutor::new(exec_addr, client.clone());

    let targets: Vec<Address> = vec![]; // fill from route
    let data: Vec<Bytes> = vec![];      // fill from calldata builder
    let min_return = U256::from(0u64);

    let tx = exec
        .execute(targets, data, min_return)
        .gas(1_500_000u64)
        .send()
        .await?;

    println!("Sent arb tx: {:?}", tx.tx_hash());
    Ok(())
}
