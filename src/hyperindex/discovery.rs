use anyhow::{anyhow, Result};
use ethers::types::Address;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tokio::time::{sleep, Duration};

use crate::engine::registry::PairRegistry;
use crate::types::{ChainConfig, PairMeta};
use crate::ui::UiHandle;

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn chain_key(prefix: &str, chain: &str) -> String {
    format!("{}_{}", prefix, chain.to_uppercase())
}

fn checkpoint_path(chain_name: &str) -> String {
    format!("data/hyperindex_checkpoint_{}.json", chain_name)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HyperIndexCheckpoint {
    /// One of: "id_cursor", "block_cursor", "offset", "full"
    mode: String,

    /// For id cursor paging.
    last_id: Option<i64>,

    /// For block cursor paging (some schemas use block_number or blockNumber).
    last_block: Option<u64>,

    /// For offset paging.
    offset: usize,

    /// Page size last used (informational).
    page_size: usize,
}

impl Default for HyperIndexCheckpoint {
    fn default() -> Self {
        Self {
            mode: "id_cursor".to_string(),
            last_id: None,
            last_block: None,
            offset: 0,
            page_size: 1000,
        }
    }
}

fn load_checkpoint(path: &str) -> HyperIndexCheckpoint {
    let p = Path::new(path);
    if let Ok(s) = fs::read_to_string(p) {
        if let Ok(cp) = serde_json::from_str::<HyperIndexCheckpoint>(&s) {
            return cp;
        }
    }
    HyperIndexCheckpoint::default()
}

fn save_checkpoint(path: &str, cp: &HyperIndexCheckpoint) -> Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = format!("{}.tmp", path);
    fs::write(&tmp, serde_json::to_string_pretty(cp)?)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[derive(Debug, Clone)]
struct PairRow {
    pair: String,
    token0: String,
    token1: String,
    dex: String,

    // optional fields for paging
    id: Option<i64>,
    block_number: Option<u64>,
    blockNumber: Option<u64>,
}

fn parse_rows(data: &Value) -> Result<Vec<PairRow>> {
    // expected shapes:
    // data.pairCreateds: [...]
    // data.pair_createds: [...]
    let arr = data
        .get("pairCreateds")
        .or_else(|| data.get("pair_createds"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("no pairCreateds/pair_createds array in response data"))?;

    let mut out = Vec::with_capacity(arr.len());
    for v in arr {
        let pair = v
            .get("pair")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow!("row missing pair"))?
            .to_string();
        let token0 = v
            .get("token0")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow!("row missing token0"))?
            .to_string();
        let token1 = v
            .get("token1")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow!("row missing token1"))?
            .to_string();
        let dex = v
            .get("dex")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow!("row missing dex"))?
            .to_string();

        let id = v.get("id").and_then(|x| x.as_i64());
        let block_number = v.get("block_number").and_then(|x| x.as_u64());
        let blockNumber = v.get("blockNumber").and_then(|x| x.as_u64());

        out.push(PairRow {
            pair,
            token0,
            token1,
            dex,
            id,
            block_number,
            blockNumber,
        });
    }
    Ok(out)
}

async fn gql_post(client: &Client, url: &str, query: &str, variables: Value) -> Result<Value> {
    let resp = client
        .post(url)
        .timeout(Duration::from_secs(20))
        .json(&json!({ "query": query, "variables": variables }))
        .send()
        .await?;

    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or_else(|_| json!({}));

    if !status.is_success() {
        return Err(anyhow!("GraphQL HTTP {} body={}", status, body));
    }

    if let Some(errs) = body.get("errors") {
        return Err(anyhow!("GraphQL errors: {}", errs));
    }

    body.get("data")
        .cloned()
        .ok_or_else(|| anyhow!("GraphQL response missing data"))
}

/// Try multiple paging query shapes, downgrading automatically if the endpoint rejects them.
///
/// Strategy:
/// 1) Cursor by id (where id > last_id, order_by id asc, limit)
/// 2) Cursor by block_number (where block_number > last_block, order_by block_number asc, limit)
/// 3) Cursor by blockNumber (where blockNumber > last_block, order_by blockNumber asc, limit)
/// 4) Offset paging (limit/offset, order_by id asc if possible)
/// 5) Full fetch (no args)
async fn fetch_next_page(
    chain: &ChainConfig,
    client: &Client,
    cp: &HyperIndexCheckpoint,
    page_size: usize,
) -> Result<(Vec<PairRow>, HyperIndexCheckpoint)> {
    let url = chain.hyperindex_url.as_str();

    // Helper to build selection set; we always *try* including id + block fields.
    let selection = "pair token0 token1 dex id block_number blockNumber";

    // 1) id cursor
    if cp.mode == "id_cursor" {
        let last = cp.last_id.unwrap_or(0);
        let q = format!(
            "query($last: Int!, $limit: Int!) {{
              pairCreateds(where: {{id: {{_gt: $last}}}}, order_by: {{id: asc}}, limit: $limit) {{
                {selection}
              }}
            }}"
        );
        match gql_post(client, url, &q, json!({ "last": last, "limit": page_size })).await {
            Ok(data) => {
                let rows = parse_rows(&data)?;
                let mut next = cp.clone();
                next.mode = "id_cursor".to_string();
                next.page_size = page_size;
                if let Some(max_id) = rows.iter().filter_map(|r| r.id).max() {
                    next.last_id = Some(max_id);
                }
                return Ok((rows, next));
            }
            Err(e) => {
                // Downgrade to next mode
                let mut next = cp.clone();
                next.mode = "block_cursor".to_string();
                next.page_size = page_size;
                return Ok((Vec::new(), next)).and_then(|_| Err(e));
            }
        }
    }

    // 2) block_number cursor
    if cp.mode == "block_cursor" {
        let lastb = cp.last_block.unwrap_or(0);
        let q = format!(
            "query($last: bigint!, $limit: Int!) {{
              pairCreateds(where: {{block_number: {{_gt: $last}}}}, order_by: {{block_number: asc}}, limit: $limit) {{
                {selection}
              }}
            }}"
        );
        match gql_post(client, url, &q, json!({ "last": lastb as i64, "limit": page_size })).await {
            Ok(data) => {
                let rows = parse_rows(&data)?;
                let mut next = cp.clone();
                next.mode = "block_cursor".to_string();
                next.page_size = page_size;
                let maxb = rows
                    .iter()
                    .filter_map(|r| r.block_number.or(r.blockNumber))
                    .max();
                if let Some(m) = maxb {
                    next.last_block = Some(m);
                }
                return Ok((rows, next));
            }
            Err(_e1) => {
                // 3) blockNumber cursor (camelCase)
                let q2 = format!(
                    "query($last: bigint!, $limit: Int!) {{
                      pairCreateds(where: {{blockNumber: {{_gt: $last}}}}, order_by: {{blockNumber: asc}}, limit: $limit) {{
                        {selection}
                      }}
                    }}"
                );
                match gql_post(client, url, &q2, json!({ "last": lastb as i64, "limit": page_size })).await {
                    Ok(data) => {
                        let rows = parse_rows(&data)?;
                        let mut next = cp.clone();
                        next.mode = "block_cursor".to_string();
                        next.page_size = page_size;
                        let maxb = rows
                            .iter()
                            .filter_map(|r| r.blockNumber.or(r.block_number))
                            .max();
                        if let Some(m) = maxb {
                            next.last_block = Some(m);
                        }
                        return Ok((rows, next));
                    }
                    Err(_e2) => {
                        // Downgrade to offset
                        let mut next = cp.clone();
                        next.mode = "offset".to_string();
                        next.page_size = page_size;
                        return Err(anyhow!("block cursor paging not supported; switching to offset"));
                    }
                }
            }
        }
    }

    // 4) offset paging
    if cp.mode == "offset" {
        let off = cp.offset;
        let q = format!(
            "query($limit: Int!, $offset: Int!) {{
              pairCreateds(limit: $limit, offset: $offset, order_by: {{id: asc}}) {{
                {selection}
              }}
            }}"
        );
        match gql_post(client, url, &q, json!({ "limit": page_size, "offset": off as i64 })).await {
            Ok(data) => {
                let rows = parse_rows(&data)?;
                let mut next = cp.clone();
                next.mode = "offset".to_string();
                next.page_size = page_size;
                next.offset = off.saturating_add(rows.len());
                return Ok((rows, next));
            }
            Err(_e) => {
                // Try offset without order_by (some schemas reject order_by)
                let q2 = format!(
                    "query($limit: Int!, $offset: Int!) {{
                      pairCreateds(limit: $limit, offset: $offset) {{
                        {selection}
                      }}
                    }}"
                );
                match gql_post(client, url, &q2, json!({ "limit": page_size, "offset": off as i64 })).await {
                    Ok(data) => {
                        let rows = parse_rows(&data)?;
                        let mut next = cp.clone();
                        next.mode = "offset".to_string();
                        next.page_size = page_size;
                        next.offset = off.saturating_add(rows.len());
                        return Ok((rows, next));
                    }
                    Err(_e2) => {
                        // Downgrade to full
                        let mut next = cp.clone();
                        next.mode = "full".to_string();
                        next.page_size = page_size;
                        return Err(anyhow!("offset paging not supported; switching to full fetch"));
                    }
                }
            }
        }
    }

    // 5) full fetch (last resort)
    let q = format!(
        "query {{
          pairCreateds {{
            {selection}
          }}
        }}"
    );
    let data = gql_post(client, url, &q, json!({})).await?;
    let rows = parse_rows(&data)?;
    let mut next = cp.clone();
    next.mode = "full".to_string();
    next.page_size = page_size;
    Ok((rows, next))
}

/// Poll HyperIndex for PairCreated events and update the registry.
/// Uses checkpointed pagination where supported.
pub async fn run_hyperindex_discovery(
    chain: ChainConfig,
    registry: PairRegistry,
    client: Client,
    ui: Option<UiHandle>,
) -> Result<()> {
    if !chain.enabled {
        tracing::info!("HyperIndex discovery disabled for chain {}", chain.name);
        return Ok(());
    }

    let target = env_usize(&chain_key("HYPERINDEX_TARGET_POOLS", &chain.name), 0);
    let page_size = env_usize(&chain_key("HYPERINDEX_PAGE_SIZE", &chain.name), 1000).max(1);

    if let Some(ui) = &ui {
        let fallback_target = env_usize(&chain_key("UNIVERSE_MAX_POOLS", &chain.name), 5000);
        let t = if target == 0 { fallback_target } else { target };
        ui.init_chain(&chain.name, chain.enabled, t).await;
        ui.set_hyperindex_progress(&chain.name, 0, "starting").await;
    }

    tracing::info!("Starting HyperIndex discovery for {}", chain.name);

    let cp_path = checkpoint_path(&chain.name);
    let mut cp = load_checkpoint(&cp_path);

    // If the operator forces a reset:
    if std::env::var(chain_key("HYPERINDEX_RESET", &chain.name)).ok().as_deref() == Some("1") {
        cp = HyperIndexCheckpoint::default();
        let _ = save_checkpoint(&cp_path, &cp);
        tracing::warn!("HyperIndex checkpoint reset for {}", chain.name);
    }

    loop {
        let loaded_before = registry.by_chain_id(chain.chain_id).len();

        // We attempt one "page". If the current mode fails, we downgrade mode and retry next loop.
        match fetch_next_page(&chain, &client, &cp, page_size).await {
            Ok((rows, next_cp)) => {
                let mut inserted = 0usize;

                for r in rows {
                    let pool: Address = match r.pair.parse() {
                        Ok(x) => x,
                        Err(_) => continue,
                    };
                    let token0: Address = match r.token0.parse() {
                        Ok(x) => x,
                        Err(_) => continue,
                    };
                    let token1: Address = match r.token1.parse() {
                        Ok(x) => x,
                        Err(_) => continue,
                    };

                    registry.insert(PairMeta {
                        chain_id: chain.chain_id,
                        chain: chain.name.clone(),
                        dex: r.dex.clone(),
                        pool,
                        token0,
                        token1,
                        fee_tier: None,
                    });
                    inserted += 1;
                }

                cp = next_cp;
                let _ = save_checkpoint(&cp_path, &cp);

                let loaded_after = registry.by_chain_id(chain.chain_id).len();
                if let Some(ui) = &ui {
                    ui.set_hyperindex_progress(&chain.name, loaded_after, "loading").await;
                    if target > 0 && loaded_after >= target {
                        ui.set_hyperindex_progress(&chain.name, loaded_after, "ready").await;
                    }
                }

                tracing::info!(
                    "HyperIndex {} mode={} page_size={} inserted={} pools={} (+{})",
                    chain.name,
                    cp.mode,
                    page_size,
                    inserted,
                    loaded_after,
                    loaded_after.saturating_sub(loaded_before)
                );

                // If cursor/offset paging returns no new rows, slow down.
                if loaded_after == loaded_before || inserted == 0 {
                    sleep(Duration::from_secs(15)).await;
                } else {
                    sleep(Duration::from_secs(2)).await;
                }
            }
            Err(e) => {
                // Downgrade mode progressively if needed.
                tracing::warn!("HyperIndex page fetch error on {}: {:?} (mode={})", chain.name, e, cp.mode);

                cp.mode = match cp.mode.as_str() {
                    "id_cursor" => "block_cursor",
                    "block_cursor" => "offset",
                    "offset" => "full",
                    _ => "full",
                }
                .to_string();

                let _ = save_checkpoint(&cp_path, &cp);

                if let Some(ui) = &ui {
                    let loaded = registry.by_chain_id(chain.chain_id).len();
                    ui.set_hyperindex_progress(&chain.name, loaded, "error").await;
                }

                sleep(Duration::from_secs(10)).await;
            }
        }
    }
}
