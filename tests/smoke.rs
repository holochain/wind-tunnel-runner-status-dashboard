use axum_test::TestServer;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use wind_tunnel_runner_status_dashboard::{AppState, build_router};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Spin up a mock Nomad API and a configured app server.
///
/// `list_json` is the response for `GET /v1/nodes`. For every node ID present
/// in that list a `GET /v1/node/:id` detail mock is also registered. By default
/// a node's detail returns empty `Meta`; entries in `detail_overrides` supply a
/// specific detail body (keyed by node ID) for nodes whose metadata matters to
/// the test.
async fn setup_test_server(
    list_json: serde_json::Value,
    detail_overrides: &[(&str, serde_json::Value)],
) -> (Arc<AppState>, TestServer) {
    // Setup mock Nomad API server
    let mock_server = MockServer::start().await;

    // Mock the /v1/nodes endpoint with test data
    Mock::given(method("GET"))
        .and(path("/v1/nodes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&list_json))
        .mount(&mock_server)
        .await;

    // Mock a /v1/node/:id detail endpoint for every node ID in the list.
    let overrides: HashMap<&str, &serde_json::Value> = detail_overrides
        .iter()
        .map(|(id, body)| (*id, body))
        .collect();
    if let Some(nodes) = list_json.as_array() {
        let mut registered = HashSet::new();
        for node in nodes {
            let Some(id) = node.get("ID").and_then(|v| v.as_str()) else {
                continue;
            };
            if !registered.insert(id.to_string()) {
                continue;
            }
            let body = overrides
                .get(id)
                .map(|v| (*v).clone())
                .unwrap_or_else(|| serde_json::json!({ "Meta": {} }));
            Mock::given(method("GET"))
                .and(path(format!("/v1/node/{id}")))
                .respond_with(ResponseTemplate::new(200).set_body_json(body))
                .mount(&mock_server)
                .await;
        }
    }

    // Create app state with mock Nomad URL
    let state = Arc::new(AppState::new(mock_server.uri(), None, false, 60));

    // Update clients list from mock API
    wind_tunnel_runner_status_dashboard::nomad::update_clients(state.clone()).await;

    // Create test server
    let app = build_router(state.clone());
    let server = TestServer::new(app);

    (state, server)
}

#[tokio::test]
async fn test_clients_list_populated() {
    let response_json = serde_json::json!([
        { "ID": "id-1", "Name": "client-1", "Status": "ready", "CreateIndex": 1 },
        { "ID": "id-2", "Name": "client-2", "Status": "ready", "CreateIndex": 1 },
        { "ID": "id-3", "Name": "client-3", "Status": "initializing", "CreateIndex": 1 }
    ]);
    let (state, _server) = setup_test_server(response_json, &[]).await;

    let clients = state.clients.read().unwrap();
    assert_eq!(clients.len(), 3, "Expected 3 clients to be populated");
    assert_eq!(
        clients.get("client-1").map(|c| c.status.as_str()),
        Some("ready")
    );
    assert_eq!(
        clients.get("client-2").map(|c| c.status.as_str()),
        Some("ready")
    );
    assert_eq!(
        clients.get("client-3").map(|c| c.status.as_str()),
        Some("initializing")
    );
}

#[tokio::test]
async fn test_clients_list_removes_duplicates() {
    let response_json = serde_json::json!([
        { "ID": "id-a", "Name": "client-1", "Status": "other", "CreateIndex": 2 },
        { "ID": "id-b", "Name": "client-1", "Status": "down", "CreateIndex": 1 },
        { "ID": "id-c", "Name": "client-2", "Status": "down", "CreateIndex": 4 },
        { "ID": "id-d", "Name": "client-2", "Status": "ready", "CreateIndex": 3 },
        { "ID": "id-e", "Name": "client-1", "Status": "ready", "CreateIndex": 6 },
        { "ID": "id-f", "Name": "client-3", "Status": "initializing", "CreateIndex": 5 }
    ]);
    let (state, _server) = setup_test_server(response_json, &[]).await;

    let clients = state.clients.read().unwrap();
    assert_eq!(clients.len(), 3, "Expected 3 clients to be populated");
    assert_eq!(
        clients.get("client-1").map(|c| c.status.as_str()),
        Some("ready")
    );
    assert_eq!(
        clients.get("client-2").map(|c| c.status.as_str()),
        Some("down")
    );
    assert_eq!(
        clients.get("client-3").map(|c| c.status.as_str()),
        Some("initializing")
    );
}

#[tokio::test]
async fn test_nonexistent_client() {
    let response_json = serde_json::json!([
        { "ID": "id-1", "Name": "client-1", "Status": "ready", "CreateIndex": 1 },
        { "ID": "id-2", "Name": "client-2", "Status": "ready", "CreateIndex": 1 },
        { "ID": "id-3", "Name": "client-3", "Status": "initializing", "CreateIndex": 1 }
    ]);
    let (_state, server) = setup_test_server(response_json, &[]).await;

    let response = server.get("/status?hostname=nonexistent-client").await;
    response.assert_status_ok();
    let body = response.text();
    assert!(
        body.contains("Not connected"),
        "Expected 'Not connected' for non-existent client"
    );
    assert!(
        body.contains("nonexistent-client"),
        "Expected hostname to be displayed"
    );
}

#[tokio::test]
async fn test_existing_client_ready_status() {
    let response_json = serde_json::json!([
        { "ID": "id-1", "Name": "client-1", "Status": "ready", "CreateIndex": 1 },
        { "ID": "id-2", "Name": "client-2", "Status": "ready", "CreateIndex": 1 },
        { "ID": "id-3", "Name": "client-3", "Status": "initializing", "CreateIndex": 1 }
    ]);
    let (_state, server) = setup_test_server(response_json, &[]).await;

    let response = server.get("/status?hostname=client-1").await;
    response.assert_status_ok();
    let body = response.text();
    assert!(
        body.contains("Ready"),
        "Expected 'Ready' status for client-1"
    );
    assert!(
        body.contains("client-1"),
        "Expected hostname to be displayed"
    );
    assert!(
        body.contains("green"),
        "Expected green background for ready status"
    );
}

#[tokio::test]
async fn test_existing_client_non_ready_status() {
    let response_json = serde_json::json!([
        { "ID": "id-1", "Name": "client-1", "Status": "ready", "CreateIndex": 1 },
        { "ID": "id-2", "Name": "client-2", "Status": "ready", "CreateIndex": 1 },
        { "ID": "id-3", "Name": "client-3", "Status": "initializing", "CreateIndex": 1 }
    ]);
    let (_state, server) = setup_test_server(response_json, &[]).await;

    let response = server.get("/status?hostname=client-3").await;
    response.assert_status_ok();
    let body = response.text();
    assert!(
        body.contains("initializing"),
        "Expected 'initializing' status for client-3"
    );
    assert!(
        body.contains("client-3"),
        "Expected hostname to be displayed"
    );
}

#[tokio::test]
async fn test_hostname_html_escaping() {
    let response_json = serde_json::json!([
        { "ID": "id-1", "Name": "client-1", "Status": "ready", "CreateIndex": 1 },
        { "ID": "id-2", "Name": "client-2", "Status": "ready", "CreateIndex": 1 },
        { "ID": "id-3", "Name": "client-3", "Status": "initializing", "CreateIndex": 1 }
    ]);
    let (_state, server) = setup_test_server(response_json, &[]).await;

    // URL-encoded version of "<script>alert('xss')</script>"
    let malicious_hostname_encoded = "%3Cscript%3Ealert%28%27xss%27%29%3C%2Fscript%3E";
    let response = server
        .get(&format!("/status?hostname={}", malicious_hostname_encoded))
        .await;
    response.assert_status_ok();
    let body = response.text();

    // Verify the script tag is escaped and not executable
    // askama_escape uses numeric character references (&#60; = <, &#62; = >, &#39; = ')
    assert!(!body.contains("<script>"), "Script tag should be escaped");
    assert!(
        body.contains("&#60;script&#62;"),
        "Expected HTML-escaped script tag"
    );
    assert!(
        body.contains("Not connected"),
        "Expected 'Not connected' status"
    );
}

#[tokio::test]
async fn test_status_html_escaping() {
    let response_json = serde_json::json!([
        { "ID": "id-1", "Name": "client-1", "Status": "<script>alert('xss')</script>", "CreateIndex": 1 }
    ]);
    let (_state, server) = setup_test_server(response_json, &[]).await;

    // URL-encoded version of "<script>alert('xss')</script>"
    let response = server.get("/status?hostname=client-1").await;
    response.assert_status_ok();
    let body = response.text();

    // Verify the script tag is escaped and not executable
    // askama_escape uses numeric character references (&#60; = <, &#62; = >, &#39; = ')
    assert!(!body.contains("<script>"), "Script tag should be escaped");

    assert!(
        !body.contains("<script>"),
        "Script tag in status should be escaped"
    );
    assert!(
        body.contains("&#60;script&#62;"),
        "Expected HTML-escaped script tag in status"
    );
    assert!(
        body.contains("client-1"),
        "Expected hostname to be displayed"
    );
}

#[tokio::test]
async fn client_meta_unyt_agent_id_is_displayed() {
    let response_json = serde_json::json!([
        { "ID": "node-1", "Name": "client-1", "Status": "ready", "CreateIndex": 1 }
    ]);
    let (_state, server) = setup_test_server(
        response_json,
        &[(
            "node-1",
            serde_json::json!({ "Meta": { "unyt_agent_id": "agent-abc-123" } }),
        )],
    )
    .await;

    let response = server.get("/status?hostname=client-1").await;
    response.assert_status_ok();
    let body = response.text();
    assert!(
        body.contains("agent-abc-123"),
        "Expected unyt_agent_id to be displayed, body was:\n{body}"
    );
}

#[tokio::test]
async fn client_meta_unyt_agent_id_is_html_escaped() {
    let response_json = serde_json::json!([
        { "ID": "node-1", "Name": "client-1", "Status": "ready", "CreateIndex": 1 }
    ]);
    let (_state, server) = setup_test_server(
        response_json,
        &[(
            "node-1",
            serde_json::json!({ "Meta": { "unyt_agent_id": "<script>alert('xss')</script>" } }),
        )],
    )
    .await;

    let response = server.get("/status?hostname=client-1").await;
    response.assert_status_ok();
    let body = response.text();
    assert!(
        !body.contains("<script>"),
        "Agent id script tag should be escaped"
    );
    assert!(
        body.contains("&#60;script&#62;"),
        "Expected HTML-escaped agent id"
    );
}

#[tokio::test]
async fn client_with_no_unyt_agent_id_shows_unknown() {
    let response_json = serde_json::json!([
        { "ID": "node-1", "Name": "client-1", "Status": "ready", "CreateIndex": 1 }
    ]);
    // No detail override -> node returns empty Meta.
    let (_state, server) = setup_test_server(response_json, &[]).await;

    let response = server.get("/status?hostname=client-1").await;
    response.assert_status_ok();
    let body = response.text();
    assert!(
        body.contains("Unknown"),
        "Expected 'Unknown' agent id when meta is absent, body was:\n{body}"
    );
}

#[tokio::test]
async fn client_with_unparseable_detail_shows_unknown() {
    let response_json = serde_json::json!([
        { "ID": "node-1", "Name": "client-1", "Status": "ready", "CreateIndex": 1 }
    ]);
    // A detail body that does not deserialize into a node detail should fail the
    // best-effort metadata fetch, leaving the client listed with empty metadata
    // rather than dropping it.
    let (_state, server) = setup_test_server(
        response_json,
        &[("node-1", serde_json::json!("not-an-object"))],
    )
    .await;

    let response = server.get("/status?hostname=client-1").await;
    response.assert_status_ok();
    let body = response.text();
    assert!(
        body.contains("Ready"),
        "Expected client to remain listed despite detail fetch failure"
    );
    assert!(
        body.contains("Unknown"),
        "Expected fallback 'Unknown' agent id, body was:\n{body}"
    );
}

#[tokio::test]
async fn all_clients_populated_when_node_count_exceeds_fetch_limit() {
    // Use more nodes than the metadata fan-out concurrency limit so the
    // semaphore-bounded path is exercised: every client must still resolve with
    // its metadata (no dropped permits and no deadlock).
    let node_count = 50usize;
    let nodes: Vec<serde_json::Value> = (0..node_count)
        .map(|i| {
            serde_json::json!({
                "ID": format!("node-{i}"),
                "Name": format!("client-{i}"),
                "Status": "ready",
                "CreateIndex": 1
            })
        })
        .collect();
    let ids: Vec<String> = (0..node_count).map(|i| format!("node-{i}")).collect();
    let overrides: Vec<(&str, serde_json::Value)> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            (
                id.as_str(),
                serde_json::json!({ "Meta": { "unyt_agent_id": format!("agent-{i}") } }),
            )
        })
        .collect();

    let (state, _server) = setup_test_server(serde_json::Value::Array(nodes), &overrides).await;

    let clients = state.clients.read().unwrap();
    assert_eq!(
        clients.len(),
        node_count,
        "Every client should be populated"
    );
    assert_eq!(
        clients
            .get("client-7")
            .and_then(|c| c.meta.get("unyt_agent_id"))
            .map(String::as_str),
        Some("agent-7"),
        "Each client should carry its fetched metadata"
    );
}
