use crate::AppState;
use chrono::Utc;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Serialize, Deserialize)]
pub(crate) struct NomadNode {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Status")]
    status: String,
    #[serde(rename = "CreateIndex")]
    create_index: u128,
}

/// Update cache with latest data from Nomad
pub async fn update_clients(state: Arc<AppState>) {
    match fetch_clients(
        state.nomad_url.clone(),
        state.nomad_token.clone(),
        state.nomad_accept_invalid_cert,
    )
    .await
    {
        Ok(mut nodes) => {
            let Ok(mut clients) = state.clients.write() else {
                log::error!("clients write lock poisoned, skipping update");
                return;
            };
            let Ok(mut last_updated) = state.last_updated.write() else {
                log::error!("last_updated write lock poisoned, skipping update");
                return;
            };

            // Regenerate list of clients
            clients.clear();

            // The nomad api can return multiple nodes with the same hostname. This can occur when a user stops and recreates their nomad agent.
            //
            // We include only the most recently created node for each hostname.
            //
            // Note that this will exclude real information if multiple people create nodes with identical hostnames.
            nodes.sort_by_key(|node| node.name.clone());
            for (name, dupe_nodes) in nodes
                .into_iter()
                .chunk_by(|node| node.name.clone())
                .into_iter()
            {
                if let Some(latest_node) = dupe_nodes.sorted_by_key(|node| node.create_index).last()
                {
                    clients.insert(name, latest_node.status);
                }
            }

            // Set last updated timestamp
            *last_updated = Utc::now();

            log::info!("Updated client list with {} clients", clients.len());
        }
        Err(e) => {
            log::error!("Failed to fetch clients from Nomad: {}", e);
        }
    }
}

// Fetch clients from Nomad API
async fn fetch_clients(
    nomad_url: String,
    nomad_token: Option<String>,
    nomad_accept_invalid_cert: bool,
) -> Result<Vec<NomadNode>, Box<dyn std::error::Error>> {
    log::info!("Fetching clients from Nomad API");

    // Build request
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(nomad_accept_invalid_cert)
        .build()?;
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
