use axum::extract::{State, Path};
use axum::response::{Html, IntoResponse, Result};
use axum::http::StatusCode;
use crate::AppState;
use std::sync::Arc;
use askama_escape::escape_html;

pub(crate) async fn home() -> Html<String> {
    Html(
        r#"
        <!DOCTYPE html>
        <html>
            <head>
                <title>Wind Tunnel Runners Status</title>
                <link rel="stylesheet" href="/static/style.css">
            </head>
            <body>
                <div class="container">
                    <h1>Wind Tunnel Runners</h1>
                    <form action="/" method="get" onsubmit="event.preventDefault(); window.location.href = '/' + document.getElementById('hostname').value;">
                        <div class="form-group">
                            <label for="hostname">Enter hostname:</label>
                            <input type="text" id="hostname" name="hostname" required>
                        </div>
                        <input type="submit" value="Check Status" class="btn">
                    </form>
                </div>
            </body>
        </html>
    "#.to_string())
}

pub(crate) async fn get_client_status(
    State(state): State<Arc<AppState>>,
    Path(hostname): Path<String>,
) -> Result<Html<String>> {
    let clients = state.clients.read().expect("Poisoned");
    let last_updated = state.last_updated.read().expect("Poisoned");

    // Parse hostname
    let mut hostname_escaped = String::new();
    escape_html(&mut hostname_escaped, &hostname)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid hostname".to_string()).into_response())?;
    
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
    Ok(Html(format!(r#"
        <!DOCTYPE html>
        <html>
            <head>
                <title>Wind Tunnel Runners</title>
                <link rel="stylesheet" href="/static/style.css">
            </head>
            <body>
                <div class="container container-wide">
                    <h1>Wind Tunnel Runners</h1>

                    <div class="section">
                        <h2 class="section-label">Hostname</h2>
                        <div class="section-value">{hostname_escaped}</div>
                    </div>

                    <div class="section">
                        <h2 class="section-label">Status</h2>
                        <div class="status-badge" style="background-color: {status_background_color};">{status_label}</div>
                    </div>

                    <div class="section">
                        <h5 class="last-updated">Last Updated</h5>
                        <div class="last-updated-value">{}</div>
                        <div class="last-updated-value">Updated every {} seconds.</div>
                    </div>

                    <a href="/" class="btn">Back to home</a>
                </div>
            </body>
        </html>
    "#, last_updated.format("%Y-%m-%d %H:%M:%S UTC"), state.update_seconds)))
}