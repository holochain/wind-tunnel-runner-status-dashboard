use std::sync::Arc;
use std::time::Duration;
use nomad_clients_status::{AppState, build_router, nomad::update_clients};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // Get settings from env variables
    let nomad_url = std::env::var("NOMAD_URL")?;
    let nomad_token = std::env::var("NOMAD_TOKEN").ok();
    let nomad_accept_invalid_cert = std::env::var("NOMAD_ACCEPT_INVALID_CERT").map(|c| c.parse::<bool>()).unwrap_or(Ok(false))?;
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or("0.0.0.0:3000".to_string());
    let update_seconds = std::env::var("UPDATE_SECONDS").map(|s| s.parse::<u64>()).unwrap_or(Ok(60))?;

    // Setup cache and task to update it
    let state = Arc::new(AppState::new(nomad_url, nomad_token, nomad_accept_invalid_cert, update_seconds));
    let state_clone = state.clone();
    tokio::spawn(async move {
        loop {
            update_clients(state_clone.clone()).await;
            tokio::time::sleep(Duration::from_secs(state_clone.update_seconds)).await;
        }
    });

    // Build server
    let app = build_router(state);

    // Run server
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
