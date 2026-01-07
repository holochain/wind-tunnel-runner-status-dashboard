use axum::extract::{State, Path};
use axum::response::Html;
use axum::http::StatusCode;
use crate::AppState;
use std::sync::Arc;
use chrono::Utc;


pub(crate) async fn home() -> Html<String> {
    Html(
        r#"
        <!DOCTYPE html>
        <html>
            <head>
                <title>Wind Tunnel Runners Status</title>
            </head>
            <body>
                <h1>Wind Tunnel Runners</h1>
                <form action="/" method="get" onsubmit="event.preventDefault(); window.location.href = '/' + document.getElementById('hostname').value;">
                    <label for="hostname">Enter hostname:</label><br>
                    <input type="text" id="hostname" name="hostname" required><br><br>
                    <input type="submit" value="Check Status">
                </form>
            </body>
        </html>
    "#.to_string())
}

pub(crate) async fn get_client_status(
    State(state): State<Arc<AppState>>,
    Path(hostname): Path<String>,
) -> Html<String> {
    let clients = state.clients.read().expect("Poisoned");
    let last_updated = state.last_updated.read().expect("Poisoned");
    let last_updated_s_ago = Utc::now().signed_duration_since(*last_updated).num_seconds();

    // Parse client status
    let (status_label, status_background_color) = match clients.get(&hostname) {
        Some(status) => {
            if status == "ready" {
                ("Ready", "green")
            } else {
                (status.as_str(), "white")
            }
        },
        None => ("Not connected", "red")
    };

    // Render status page
    Html(format!(r#"
        <!DOCTYPE html>
        <html>
            <head>
                <title>Wind Tunnel Runners</title>
            </head>
            <body>
                <h1>Wind Tunnel Runners</h1>
                
                <div style="margin-top: 25px; margin-bottom: 25px;">
                    <h2>Hostname</h2>
                    {hostname}
                </div>
                
                <div style="margin-bottom: 25px;">
                    <h2>Status</h2>
                    <div style="display: inline-block; background-color: {status_background_color}; text: white; padding: 4px; border-radius: 10px;">{status_label}</div>
                </div>

                <div style="margin-bottom: 25px;">
                    <h5>Last Updated</h5>
                    {last_updated_s_ago} seconds ago. Updated every {} seconds.
                </div>
                
                <a href="/">Back to home</a>
            </body>
        </html>
    "#, state.update_seconds))
}