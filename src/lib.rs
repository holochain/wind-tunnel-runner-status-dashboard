use axum::{
    Router,
    routing::get,
};
use tower_http::services::ServeDir;
use std::sync::Arc;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use std::sync::RwLock;

mod routes;
use routes::{home, get_client_status};

pub mod nomad;

type ClientName = String;
type ReadyStatus = String;

pub struct AppState {
    pub clients: RwLock<HashMap<ClientName, ReadyStatus>>,
    pub last_updated: RwLock<DateTime<Utc>>,
    pub update_seconds: u64,
    pub nomad_url: String,
    pub nomad_token: Option<String>,
    pub nomad_accept_invalid_cert: bool,
}

impl AppState {
    pub fn new(nomad_url: String, nomad_token: Option<String>, nomad_accept_invalid_cert: bool, update_seconds: u64) -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
            last_updated: RwLock::new(Utc::now()),
            update_seconds,
            nomad_url,
            nomad_token,
            nomad_accept_invalid_cert
        }
    }
}

/// Build the router with app state
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(home))
        .route("/{hostname}", get(get_client_status))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state)
}
