use crate::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[derive(Serialize, Deserialize)]
struct NomadNode {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Status")]
    status: String,
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
        Ok(nodes) => {
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

            for node in nodes {
                clients.insert(node.name, node.status);
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
