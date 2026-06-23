use crate::{AppState, ClientInfo};
use chrono::Utc;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Maximum number of node detail requests issued to Nomad at once. Bounds the
/// per-refresh fan-out so the request burst does not grow with the fleet size.
const MAX_CONCURRENT_DETAIL_FETCHES: usize = 20;

#[derive(Serialize, Deserialize)]
pub(crate) struct NomadNode {
    #[serde(rename = "ID")]
    id: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Status")]
    status: String,
    #[serde(rename = "CreateIndex")]
    create_index: u128,
}

/// Detail for a single node, from `GET /v1/node/:id`.
///
/// The nodes list (`GET /v1/nodes`) does not include `Meta`, so we fetch the
/// per-node detail to obtain it.
#[derive(Deserialize)]
struct NomadNodeDetail {
    #[serde(rename = "Meta", default)]
    meta: Option<HashMap<String, String>>,
}

/// Update cache with latest data from Nomad
pub async fn update_clients(state: Arc<AppState>) {
    // Build a single client to reuse across the nodes list request and all of
    // the per-node detail requests.
    let Ok(client) = reqwest::Client::builder()
        .danger_accept_invalid_certs(state.nomad_accept_invalid_cert)
        .build()
        .inspect_err(|e| log::error!("Failed to build HTTP client: {}", e))
    else {
        return;
    };

    let Ok(nodes) = fetch_clients(&client, &state.nomad_url, state.nomad_token.as_deref())
        .await
        .inspect_err(|e| log::error!("Failed to fetch clients from Nomad: {}", e))
    else {
        return;
    };

    // The nomad api can return multiple nodes with the same hostname. This can occur when a user stops and recreates their nomad agent.
    //
    // We include only the most recently created node for each hostname.
    //
    // Note that this will exclude real information if multiple people create nodes with identical hostnames.
    let mut nodes = nodes;
    nodes.sort_by_key(|node| node.name.clone());
    let latest_nodes: Vec<NomadNode> = nodes
        .into_iter()
        .chunk_by(|node| node.name.clone())
        .into_iter()
        .filter_map(|(_, dupe_nodes)| dupe_nodes.sorted_by_key(|node| node.create_index).last())
        .collect();

    // Fetch each retained node's metadata concurrently, capping the number of
    // in-flight requests with a semaphore. A failed detail fetch is non-fatal:
    // the client is still listed, just without metadata.
    let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_DETAIL_FETCHES));
    let mut fetches = tokio::task::JoinSet::new();
    for node in latest_nodes {
        let client = client.clone();
        let nomad_url = state.nomad_url.clone();
        let nomad_token = state.nomad_token.clone();
        let semaphore = semaphore.clone();
        fetches.spawn(async move {
            // Hold a permit for the duration of the fetch to bound concurrency.
            // The semaphore is never closed, so acquisition cannot fail.
            let _permit = semaphore.acquire_owned().await.ok();
            let meta = fetch_node_meta(&client, &nomad_url, nomad_token.as_deref(), &node.id).await;
            (
                node.name,
                ClientInfo {
                    status: node.status,
                    meta,
                },
            )
        });
    }

    let mut new_clients = HashMap::new();
    while let Some(result) = fetches.join_next().await {
        match result {
            Ok((name, info)) => {
                new_clients.insert(name, info);
            }
            Err(e) => log::error!("Node metadata task failed to complete: {}", e),
        }
    }

    // Commit the freshly built client list to the cache.
    let Ok(mut clients) = state.clients.write() else {
        log::error!("clients write lock poisoned, skipping update");
        return;
    };
    let Ok(mut last_updated) = state.last_updated.write() else {
        log::error!("last_updated write lock poisoned, skipping update");
        return;
    };

    let client_count = new_clients.len();
    *clients = new_clients;
    *last_updated = Utc::now();

    log::info!("Updated client list with {} clients", client_count);
}

// Fetch clients from Nomad API
async fn fetch_clients(
    client: &reqwest::Client,
    nomad_url: &str,
    nomad_token: Option<&str>,
) -> Result<Vec<NomadNode>, Box<dyn std::error::Error>> {
    log::info!("Fetching clients from Nomad API");

    // Build request
    let mut request_builder = client
        .get(format!("{nomad_url}/v1/nodes"))
        .timeout(Duration::from_secs(10));
    if let Some(nomad_token) = nomad_token {
        request_builder = request_builder.bearer_auth(nomad_token);
    }
    let request = request_builder.build()?;

    log::debug!(
        "Sending request: {} {} {:?}",
        request.method(),
        request.url(),
        request.headers()
    );

    // Send request
    let response = client.execute(request).await?;

    // Handle response
    if !response.status().is_success() {
        return Err(format!("Nomad API returned status: {}", response.status()).into());
    }

    let nodes: Vec<NomadNode> = response.json().await?;
    log::info!("Successfully fetched {} clients from Nomad", nodes.len());

    Ok(nodes)
}

/// Fetch a single node's `Meta` map from the Nomad node detail endpoint.
///
/// Errors are logged and surfaced as an empty map so that a single failed
/// detail request does not prevent the rest of the client list from updating.
async fn fetch_node_meta(
    client: &reqwest::Client,
    nomad_url: &str,
    nomad_token: Option<&str>,
    node_id: &str,
) -> HashMap<String, String> {
    match fetch_node_detail(client, nomad_url, nomad_token, node_id).await {
        Ok(detail) => detail.meta.unwrap_or_default(),
        Err(e) => {
            log::error!("Failed to fetch metadata for node {}: {}", node_id, e);
            HashMap::new()
        }
    }
}

async fn fetch_node_detail(
    client: &reqwest::Client,
    nomad_url: &str,
    nomad_token: Option<&str>,
    node_id: &str,
) -> Result<NomadNodeDetail, Box<dyn std::error::Error>> {
    let mut request_builder = client
        .get(format!("{nomad_url}/v1/node/{node_id}"))
        .timeout(Duration::from_secs(10));
    if let Some(nomad_token) = nomad_token {
        request_builder = request_builder.bearer_auth(nomad_token);
    }
    let response = client.execute(request_builder.build()?).await?;

    if !response.status().is_success() {
        return Err(format!("Nomad API returned status: {}", response.status()).into());
    }

    Ok(response.json().await?)
}
