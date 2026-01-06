use axum::extract::State;
use axum:: response::Html;
use crate::AppState;
use std::sync::Arc;


pub(crate) async fn list_clients(
    State(state): State<Arc<AppState>>,
) -> Html<String> {
    let cache = state.clients.read().expect("Poisoned");  

    let list_items = cache.iter()
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
                <ul>{list_items}<ul>
            </body>
        </html>
    "#,))
}