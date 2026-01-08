use axum_test::TestServer;
use std::sync::Arc;
use wind_tunnel_runner_status_dashboard::{AppState, build_router};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup_test_server() -> (Arc<AppState>, TestServer) {
    // Setup mock Nomad API server
    let mock_server = MockServer::start().await;

    // Mock the /v1/nodes endpoint with test data
    Mock::given(method("GET"))
        .and(path("/v1/nodes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "Name": "client-1",
                "Status": "ready"
            },
            {
                "Name": "client-2",
                "Status": "ready"
            },
            {
                "Name": "client-3",
                "Status": "initializing"
            }
        ])))
        .mount(&mock_server)
        .await;

    // Create app state with mock Nomad URL
    let state = Arc::new(AppState::new(mock_server.uri(), None, false, 60));

    // Update clients list from mock API
    wind_tunnel_runner_status_dashboard::nomad::update_clients(state.clone()).await;

    // Create test server
    let app = build_router(state.clone());
    let server = TestServer::new(app).unwrap();

    (state, server)
}

#[tokio::test]
async fn test_clients_list_populated() {
    let (state, _server) = setup_test_server().await;

    let clients = state.clients.read().unwrap();
    assert_eq!(clients.len(), 3, "Expected 3 clients to be populated");
    assert_eq!(clients.get("client-1"), Some(&"ready".to_string()));
    assert_eq!(clients.get("client-2"), Some(&"ready".to_string()));
    assert_eq!(clients.get("client-3"), Some(&"initializing".to_string()));
}

#[tokio::test]
async fn test_nonexistent_client() {
    let (_state, server) = setup_test_server().await;

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
    let (_state, server) = setup_test_server().await;

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
    let (_state, server) = setup_test_server().await;

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
    let (_state, server) = setup_test_server().await;

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
    // Setup mock Nomad API server with malicious status
    let mock_server = MockServer::start().await;

    // Mock client with status containing HTML/script tags
    Mock::given(method("GET"))
        .and(path("/v1/nodes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {
                "Name": "client-with-xss",
                "Status": "<script>alert('xss')</script>"
            }
        ])))
        .mount(&mock_server)
        .await;

    // Create app state with mock Nomad URL
    let state = Arc::new(AppState::new(mock_server.uri(), None, false, 60));

    // Update clients list from mock API
    wind_tunnel_runner_status_dashboard::nomad::update_clients(state.clone()).await;

    // Create test server
    let app = build_router(state.clone());
    let server = TestServer::new(app).unwrap();

    let response = server.get("/status?hostname=client-with-xss").await;
    response.assert_status_ok();
    let body = response.text();

    // Verify the script tag in status is escaped and not executable
    // askama_escape uses numeric character references (&#60; = <, &#62; = >, &#39; = ')
    assert!(
        !body.contains("<script>"),
        "Script tag in status should be escaped"
    );
    assert!(
        body.contains("&#60;script&#62;"),
        "Expected HTML-escaped script tag in status"
    );
    assert!(
        body.contains("client-with-xss"),
        "Expected hostname to be displayed"
    );
}
