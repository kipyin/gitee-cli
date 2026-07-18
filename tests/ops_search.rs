use gitee_cli_rs::api::client::Client;
use gitee_cli_rs::api::search::{SearchIssuesFilter, SearchReposFilter, SearchUsersFilter};

const REPO_LIST_JSON: &str = include_str!("fixtures/repo_list.json");
const ISSUE_JSON: &str = include_str!("fixtures/issue.json");
const USER_LIST_JSON: &str = r#"[{
  "id": 1,
  "login": "kip",
  "name": "Kip Yin",
  "html_url": "https://gitee.com/kip"
}]"#;

fn client(server: &mockito::ServerGuard) -> Client {
    Client::new(format!("{}/api/v5", server.url()), "fake-token".into())
}

fn api_path(path: &str) -> String {
    format!("/api/v5{path}")
}

#[test]
fn search_repos_hits_path_with_query_keys() {
    let mut server = mockito::Server::new();
    let path = "/search/repositories";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("q".into(), "gitee".into()),
            mockito::Matcher::UrlEncoded("owner".into(), "oschina".into()),
            mockito::Matcher::UrlEncoded("language".into(), "Rust".into()),
            mockito::Matcher::UrlEncoded("fork".into(), "true".into()),
            mockito::Matcher::UrlEncoded("sort".into(), "stars_count".into()),
            mockito::Matcher::UrlEncoded("order".into(), "desc".into()),
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(REPO_LIST_JSON)
        .create();

    let filter = SearchReposFilter {
        q: "gitee",
        owner: Some("oschina"),
        language: Some("Rust"),
        fork: true,
        sort: Some("stars_count"),
        order: Some("desc"),
        limit: 5,
    };
    let items = client(&server)
        .search()
        .repos(&filter)
        .expect("search repos should succeed");

    mock.assert();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].full_name, "oschina/gitee-cli");
}

#[test]
fn search_repos_empty_array() {
    let mut server = mockito::Server::new();
    let path = "/search/repositories";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("q".into(), "gitee".into()),
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    let filter = SearchReposFilter {
        q: "gitee",
        owner: None,
        language: None,
        fork: false,
        sort: None,
        order: None,
        limit: 5,
    };
    let items = client(&server)
        .search()
        .repos(&filter)
        .expect("empty search repos should succeed");

    mock.assert();
    assert!(items.is_empty());
}

#[test]
fn search_issues_hits_path_with_query_keys() {
    let mut server = mockito::Server::new();
    let path = "/search/issues";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("q".into(), "login".into()),
            mockito::Matcher::UrlEncoded("repo".into(), "oschina/gitee-cli".into()),
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
            mockito::Matcher::UrlEncoded("author".into(), "reporter".into()),
            mockito::Matcher::UrlEncoded("assignee".into(), "dev1".into()),
            mockito::Matcher::UrlEncoded("label".into(), "bug".into()),
            mockito::Matcher::UrlEncoded("language".into(), "Rust".into()),
            mockito::Matcher::UrlEncoded("sort".into(), "updated_at".into()),
            mockito::Matcher::UrlEncoded("order".into(), "asc".into()),
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!("[{ISSUE_JSON}]"))
        .create();

    let filter = SearchIssuesFilter {
        q: "login",
        repo: Some("oschina/gitee-cli"),
        language: Some("Rust"),
        label: Some("bug"),
        state: Some("open"),
        author: Some("reporter"),
        assignee: Some("dev1"),
        sort: Some("updated_at"),
        order: Some("asc"),
        limit: 3,
    };
    let items = client(&server)
        .search()
        .issues(&filter)
        .expect("search issues should succeed");

    mock.assert();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].number, "88");
}

#[test]
fn search_issues_empty_array() {
    let mut server = mockito::Server::new();
    let path = "/search/issues";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("q".into(), "gitee".into()),
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    let filter = SearchIssuesFilter {
        q: "gitee",
        repo: None,
        language: None,
        label: None,
        state: None,
        author: None,
        assignee: None,
        sort: None,
        order: None,
        limit: 3,
    };
    let items = client(&server)
        .search()
        .issues(&filter)
        .expect("empty search issues should succeed");

    mock.assert();
    assert!(items.is_empty());
}

#[test]
fn search_users_hits_path_with_query_keys() {
    let mut server = mockito::Server::new();
    let path = "/search/users";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("q".into(), "kip".into()),
            mockito::Matcher::UrlEncoded("sort".into(), "followers_count".into()),
            mockito::Matcher::UrlEncoded("order".into(), "desc".into()),
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(USER_LIST_JSON)
        .create();

    let filter = SearchUsersFilter {
        q: "kip",
        sort: Some("followers_count"),
        order: Some("desc"),
        limit: 3,
    };
    let items = client(&server)
        .search()
        .users(&filter)
        .expect("search users should succeed");

    mock.assert();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].login, "kip");
}

#[test]
fn search_users_empty_array() {
    let mut server = mockito::Server::new();
    let path = "/search/users";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("q".into(), "nobody".into()),
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    let filter = SearchUsersFilter {
        q: "nobody",
        sort: None,
        order: None,
        limit: 3,
    };
    let items = client(&server)
        .search()
        .users(&filter)
        .expect("empty search users should succeed");

    mock.assert();
    assert!(items.is_empty());
}
