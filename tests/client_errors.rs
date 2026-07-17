use gitee::api::client::Client;
use gitee::error::GiteeError;
use gitee::models::Issue;

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
    assert_eq!(issue.state, "open");
    assert_eq!(
        issue.html_url,
        "https://gitee.com/owner/repo/issues/I42"
    );
}
