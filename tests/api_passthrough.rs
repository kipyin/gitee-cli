use gitee_cli_rs::api::client::{Client, RawRequest};
use gitee_cli_rs::error::GiteeError;

fn client(server: &mockito::ServerGuard) -> Client {
    Client::new(format!("{}/api/v5", server.url()), "fake-token".into())
}

fn api_path(path: &str) -> String {
    format!("/api/v5{path}")
}

fn page_items(count: usize) -> String {
    let items: Vec<String> = (0..count)
        .map(|i| format!(r#"{{"id":{i}}}"#))
        .collect();
    format!("[{}]", items.join(","))
}

#[test]
fn raw_get_returns_body_and_sends_auth() {
    let mut server = mockito::Server::new();
    let path = "/user";
    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_header("authorization", "token fake-token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"login":"alice"}"#)
        .create();

    let client = client(&server);
    let body = client
        .raw(&RawRequest {
            method: "GET",
            path,
            query: &[],
            form: &[],
            headers: &[],
            body: None,
        })
        .expect("raw GET should succeed");

    mock.assert();
    assert!(body.contains("alice"));
}

#[test]
fn raw_post_sends_urlencoded_form() {
    let mut server = mockito::Server::new();
    let path = "/gists";
    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_header("authorization", "token fake-token")
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("description".into(), "t".into()),
            mockito::Matcher::UrlEncoded(
                "files[x.rs][content]".into(),
                "fn main(){}".into(),
            ),
        ]))
        .with_status(201)
        .with_body(r#"{"id":"1"}"#)
        .create();

    let client = client(&server);
    client
        .raw(&RawRequest {
            method: "POST",
            path,
            query: &[],
            form: &[
                ("files[x.rs][content]", "fn main(){}"),
                ("description", "t"),
            ],
            headers: &[],
            body: None,
        })
        .expect("raw POST should succeed");

    mock.assert();
}

#[test]
fn raw_get_puts_form_on_query_string() {
    let mut server = mockito::Server::new();
    let path = "/search/issues";
    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("q".into(), "bug".into()),
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
        ]))
        .with_status(200)
        .with_body("[]")
        .create();

    let client = client(&server);
    client
        .raw(&RawRequest {
            method: "GET",
            path,
            query: &[],
            form: &[("q", "bug"), ("state", "open")],
            headers: &[],
            body: None,
        })
        .expect("raw GET with form-as-query should succeed");

    mock.assert();
}

#[test]
fn raw_non_2xx_returns_api_error_with_body() {
    let mut server = mockito::Server::new();
    let path = "/repos/o/r";
    let err_body = "validation failed: title missing";
    server
        .mock("GET", api_path(path).as_str())
        .with_status(422)
        .with_body(err_body)
        .create();

    let client = client(&server);
    let err = client
        .raw(&RawRequest {
            method: "GET",
            path,
            query: &[],
            form: &[],
            headers: &[],
            body: None,
        })
        .expect_err("expected API error");

    match err {
        GiteeError::Api { status, message } => {
            assert_eq!(status, 422);
            assert_eq!(message, err_body);
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[test]
fn raw_paged_merges_full_and_partial_pages() {
    let mut server = mockito::Server::new();
    let path = "/notifications/threads";

    let page1 = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page_items(100))
        .create();

    let page2 = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "2".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"id":100},{"id":101}]"#)
        .create();

    let client = client(&server);
    let items = client
        .raw_paged(path, &[], &[])
        .expect("raw_paged should succeed");

    page1.assert();
    page2.assert();
    assert_eq!(items.len(), 102);
    assert_eq!(items[0]["id"], 0);
    assert_eq!(items[101]["id"], 101);
}

#[test]
fn raw_paged_forwards_caller_query_params() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/issues";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
            mockito::Matcher::UrlEncoded("state".into(), "closed".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"id":1}]"#)
        .create();

    let items = client(&server)
        .raw_paged(path, &[("state", "closed")], &[])
        .expect("raw_paged should succeed");

    mock.assert();
    assert_eq!(items.len(), 1);
}

#[test]
fn raw_paged_errors_on_non_array_page() {
    let mut server = mockito::Server::new();
    let path = "/user";
    server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_body(r#"{"login":"alice"}"#)
        .create();

    let client = client(&server);
    let err = client
        .raw_paged(path, &[], &[])
        .expect_err("expected paginate usage error");

    assert!(matches!(err, GiteeError::Usage(_)));
}
