use axum::extract::State;
use axum:: response::Html;
use crate::AppState;
use std::sync::Arc;
use chrono::Utc;


pub(crate) async fn list_clients(
    State(state): State<Arc<AppState>>,
) -> Html<String> {
    let clients = state.clients.read().expect("Poisoned");  
    let last_updated = state.last_updated.read().expect("Poisoned");
    let last_updated_s_ago = Utc::now().signed_duration_since(*last_updated).num_seconds();
    
    let mut clients_list: Vec<_> = clients.clone().into_iter().collect();
    clients_list.sort_by_key(|(k, _)| k.clone());

    let clients_list_html = clients_list.iter()
        .map(|(k,v)| format!("<li><strong>{k}</strong>: {v}</li>"))
        .collect::<Vec<String>>()
        .join("\n");

    Html(format!(
        r#"
        <!DOCTYPE html>
        <html>
            <head>
                <title>Wind Tunnel Runners Status</title>
            </head>
            <body>
                <h1>Wind Tunnel Runners</h1>
                <h5>Last Updated {last_updated_s_ago} seconds ago. Updated every {} seconds.</h5>
                <ul>{clients_list_html}<ul>
            </body>
        </html>
    "#, state.update_seconds))
}