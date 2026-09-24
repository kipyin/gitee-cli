use gitee_cli_rs::api::client::{Client, RawRequest};
use gitee_cli_rs::error::GiteeError;
use gitee_cli_rs::models::{Issue, IssueState};

fn client(server: &mockito::ServerGuard) -> Client {
    Client::new(format!("{}/api/v5", server.url()), "fake-token".into())
}

fn api_path(path: &str) -> String {
    format!("/api/v5{path}")
}

#[test]
fn get_401_maps_to_unauthorized() {
    let mut server = mockito::Server::new();
    let path = "/user";
    server
        .mock("GET", api_path(path).as_str())
        .with_status(401)
        .with_body(r#"{"message":"401 Unauthorized"}"#)
        .create();

    let client = client(&server);
    let err = client
        .get::<Issue>(path, &[])
        .expect_err("expected unauthorized");

    assert!(matches!(err, GiteeError::Unauthorized));
}

#[test]
fn patch_issues_404_project_or_enterprise_maps_to_api() {
    let mut server = mockito::Server::new();
    let path = "/repos/owner/issues/I1";
    server
        .mock("PATCH", api_path(path).as_str())
        .with_status(404)
        .with_body(r#"{"message":"project or enterprise"}"#)
        .create();

    let client = client(&server);
    let err = client
        .patch_json::<Issue>(
            path,
            &serde_json::json!({"repo": "r", "title": "t", "state": "open"}),
        )
        .expect_err("expected api error");

    match err {
        GiteeError::Api { status, message } => {
            assert_eq!(status, 404);
            assert!(message.to_lowercase().contains("project or enterprise"));
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[test]
fn get_404_project_or_enterprise_stays_not_found() {
    let mut server = mockito::Server::new();
    let path = "/repos/owner/repo/issues/I1";
    server
        .mock("GET", api_path(path).as_str())
        .with_status(404)
        .with_body(r#"{"message":"project or enterprise"}"#)
        .create();

    let client = client(&server);
    let err = client
        .get::<Issue>(path, &[])
        .expect_err("expected not found");

    match err {
        GiteeError::NotFound(p) => assert_eq!(p, path),
        other => panic!("expected NotFound for non-PATCH, got {other:?}"),
    }
}

#[test]
fn get_404_maps_to_not_found_with_path() {
    let mut server = mockito::Server::new();
    let path = "/repos/owner/missing";
    server
        .mock("GET", api_path(path).as_str())
        .with_status(404)
        .with_body(r#"{"message":"Not Found"}"#)
        .create();

    let client = client(&server);
    let err = client
        .get::<Issue>(path, &[])
        .expect_err("expected not found");

    match err {
        GiteeError::NotFound(p) => assert_eq!(p, path),
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn get_422_maps_to_api_with_json_message() {
    let mut server = mockito::Server::new();
    let path = "/repos/owner/repo/issues";
    server
        .mock("GET", api_path(path).as_str())
        .with_status(422)
        .with_body(r#"{"message":"title is required"}"#)
        .create();

    let client = client(&server);
    let err = client
        .get::<Issue>(path, &[])
        .expect_err("expected api error");

    match err {
        GiteeError::Api { status, message } => {
            assert_eq!(status, 422);
            assert_eq!(message, "title is required");
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[test]
fn get_500_non_json_body_uses_trimmed_text() {
    let mut server = mockito::Server::new();
    let path = "/repos/owner/repo/issues";
    let body = format!("  {}  ", "x".repeat(350));
    let expected = format!("{}…", "x".repeat(300));
    server
        .mock("GET", api_path(path).as_str())
        .with_status(500)
        .with_body(body)
        .create();

    let client = client(&server);
    let err = client
        .get::<Issue>(path, &[])
        .expect_err("expected api error");

    match err {
        GiteeError::Api { status, message } => {
            assert_eq!(status, 500);
            assert_eq!(message, expected);
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[test]
fn get_200_deserializes_issue() {
    let mut server = mockito::Server::new();
    let path = "/repos/owner/repo/issues/1";
    let body = r#"{
        "number": "42",
        "title": "Bug report",
        "state": "open",
        "html_url": "https://gitee.com/owner/repo/issues/I42"
    }"#;
    server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

    let client = client(&server);
    let issue: Issue = client.get(path, &[]).expect("expected success");

    assert_eq!(issue.number, "42");
    assert_eq!(issue.title, "Bug report");
    assert_eq!(issue.state, IssueState::Open);
    assert_eq!(issue.html_url, "https://gitee.com/owner/repo/issues/I42");
}

/// Connect failures are `Network` on every send path, including the ones that
/// used to surface as `Http` (`get_ok`, multipart upload, asset download, `raw`).
#[test]
fn connect_failure_is_network_on_every_send() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);

    let client = Client::new(
        format!("http://127.0.0.1:{port}/api/v5"),
        "fake-token".into(),
    );
    let assert_network = |err: GiteeError| {
        assert_eq!(err.exit_code(), 6);
        assert_eq!(err.code_slug(), "network");
        let rendered = err.to_string();
        assert!(
            matches!(err, GiteeError::Network(_)),
            "expected Network, got {rendered}"
        );
    };

    assert_network(client.get::<Issue>("/user", &[]).expect_err("get"));
    assert_network(client.get_ok("/user").expect_err("get_ok"));
    assert_network(
        client
            .raw(&RawRequest {
                method: "GET",
                path: "/user",
                query: &[],
                form: &[],
                headers: &[],
                body: None,
            })
            .expect_err("raw"),
    );
    assert_network(
        client
            .get_bytes(&format!("http://127.0.0.1:{port}/asset"))
            .expect_err("get_bytes"),
    );

    let file = std::env::temp_dir().join(format!("gitee-cli-connect-{port}"));
    std::fs::write(&file, b"asset").expect("temp file");
    let uploaded = client.post_multipart::<serde_json::Value>(
        "/repos/o/r/releases/v1/attach_files",
        file.to_str().expect("utf8 path"),
    );
    let _ = std::fs::remove_file(&file);
    assert_network(uploaded.expect_err("post_multipart"));
}
