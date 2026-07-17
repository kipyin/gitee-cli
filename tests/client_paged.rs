use gitee::api::client::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct PageItem {
    id: u32,
}

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
fn get_paged_fetches_full_page_then_empty_page() {
    let mut server = mockito::Server::new();
    let path = "/repos/owner/repo/issues";

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
        .with_body("[]")
        .create();

    let client = client(&server);
    let items: Vec<PageItem> = client
        .get_paged(path, &[], 200)
        .expect("paged fetch should succeed");

    page1.assert();
    page2.assert();
    assert_eq!(items.len(), 100);
    assert_eq!(items[0], PageItem { id: 0 });
    assert_eq!(items[99], PageItem { id: 99 });
}

#[test]
fn get_paged_limit_truncates_results() {
    let mut server = mockito::Server::new();
    let path = "/repos/owner/repo/issues";

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

    let client = client(&server);
    let items: Vec<PageItem> = client
        .get_paged(path, &[], 30)
        .expect("paged fetch should succeed");

    page1.assert();
    assert_eq!(items.len(), 30);
    assert_eq!(items[29], PageItem { id: 29 });
}

#[test]
fn get_paged_increments_page_query_param() {
    let mut server = mockito::Server::new();
    let path = "/repos/owner/repo/pulls";

    let page1 = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
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
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"id":100}]"#)
        .create();

    let client = client(&server);
    let items: Vec<PageItem> = client
        .get_paged(path, &[("state", "open")], 150)
        .expect("paged fetch should succeed");

    page1.assert();
    page2.assert();
    assert_eq!(items.len(), 101);
    assert_eq!(items[100], PageItem { id: 100 });
}
