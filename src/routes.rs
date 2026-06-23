use crate::AppState;
use askama_escape::escape_html;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Result};
use serde::Deserialize;
use std::sync::Arc;

pub(crate) async fn home() -> Html<String> {
    Html(
        r#"
        <!DOCTYPE html>
        <html>
            <head>
                <title>Wind Tunnel Runners Status</title>
                <link rel="stylesheet" href="/static/style.css" />
            </head>
            <body>
                <div class="container">
                    <h1>Wind Tunnel Runners</h1>
                    <p>Enter the hostname of your Wind Tunnel Runner to check its connection status.</p>
                    <form action="/status" method="GET">
                        <div class="form-group">
                            <label for="hostname">Enter hostname:</label>
                            <input type="text" id="hostname" name="hostname" required>
                        </div>
                        <input type="submit" value="Check Status" class="btn">
                    </form>
                </div>
            </body>
        </html>
    "#
        .to_string(),
    )
}

struct StatusBadge {
    background_color: String,
    color: String,
    text: String,
}

impl StatusBadge {
    fn as_html(&self) -> String {
        format!(
            r#"
            <div class="status-badge" style="background-color: {}; color: {};">
                {}
            </div>
        "#,
            self.background_color, self.color, self.text
        )
    }
}

#[derive(Deserialize)]
pub(crate) struct HostnameParams {
    hostname: String,
}

pub(crate) async fn status(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HostnameParams>,
) -> Result<Html<String>> {
    let clients = state.clients.read().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
            .into_response()
    })?;
    let last_updated = state.last_updated.read().map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal server error".to_string(),
        )
            .into_response()
    })?;

    // Parse hostname
    let mut hostname_escaped = String::new();
    escape_html(&mut hostname_escaped, &params.hostname)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid hostname".to_string()).into_response())?;
    let hostname_parsed = hostname_escaped.as_str().trim();

    // Check if hostname is blank
    if hostname_parsed.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Hostname cannot be blank".to_string(),
        )
            .into_response()
            .into());
    }

    // Look up the cached client info for this hostname.
    let client_info = clients.get(params.hostname.as_str().trim());

    // Parse client status
    let status_badge = match client_info.map(|info| info.status.as_str()) {
        Some(status) => {
            if status == "ready" {
                StatusBadge {
                    text: "Ready".to_string(),
                    background_color: "green".to_string(),
                    color: "white".to_string(),
                }
            } else if status == "down" {
                StatusBadge {
                    text: "Down".to_string(),
                    background_color: "red".to_string(),
                    color: "white".to_string(),
                }
            } else {
                // Escape html from nomad api provided status string
                // This is likely overkill since we are running the nomad server,
                // but is good practice anyway.
                let mut status_escaped = String::new();
                escape_html(&mut status_escaped, status).map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal server error".to_string(),
                    )
                        .into_response()
                })?;

                StatusBadge {
                    text: status_escaped,
                    background_color: "white".to_string(),
                    color: "black".to_string(),
                }
            }
        }
        None => StatusBadge {
            text: "Not connected".to_string(),
            background_color: "red".to_string(),
            color: "white".to_string(),
        },
    };

    // Resolve the unyt_agent_id from the client's metadata, falling back to
    // "Unknown" when the client is not connected or has no such metadata.
    // The value is Nomad-provided, so escape it before rendering.
    let unyt_agent_id = match client_info.and_then(|info| info.meta.get("unyt_agent_id")) {
        Some(agent_id) => {
            let mut escaped = String::new();
            escape_html(&mut escaped, agent_id).map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
                    .into_response()
            })?;
            escaped
        }
        None => "Unknown".to_string(),
    };

    // Render status page
    Ok(Html(format!(
        r#"
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
                        <div class="section-value">{hostname_parsed}</div>
                    </div>

                    <div class="section">
                        <h2 class="section-label">Status</h2>
                        {}
                    </div>

                    <div class="section">
                        <h2 class="section-label">Attributed Unyt Agent</h2>
                        <div class="section-value">{unyt_agent_id}</div>
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
    "#,
        status_badge.as_html(),
        last_updated.format("%Y-%m-%d %H:%M:%S UTC"),
        state.update_seconds
    )))
}
