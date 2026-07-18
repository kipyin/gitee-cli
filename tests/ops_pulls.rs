use gitee_cli_rs::api::client::Client;
use gitee_cli_rs::api::pulls::{CreatePr, PrFilter};
use gitee_cli_rs::models::{MergeMethod, PrState};
use gitee_cli_rs::repo::Repo;

const PULL_REQUEST_JSON: &str = include_str!("fixtures/pull_request.json");
const PR_FILE_DIFF_JSON: &str = include_str!("fixtures/pr_file_diff.json");

fn client(server: &mockito::ServerGuard) -> Client {
    Client::new(format!("{}/api/v5", server.url()), "fake-token".into())
}

fn api_path(path: &str) -> String {
    format!("/api/v5{path}")
}

fn test_repo() -> Repo {
    Repo {
        owner: "oschina".to_string(),
        name: "gitee-cli".to_string(),
    }
}

#[test]
fn list_hits_pulls_path_with_state_and_author_single_page() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
            mockito::Matcher::UrlEncoded("author".into(), "dev1".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!("[{PULL_REQUEST_JSON}]"))
        .create();

    let items = client(&server)
        .pulls(&test_repo())
        .list(&PrFilter {
            state: Some("open"),
            author: Some("dev1"),
            limit: 50,
        })
        .expect("list should succeed");

    mock.assert();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].number, 12);
    assert_eq!(items[0].title, "Add pagination helpers");
}

#[test]
fn get_deserializes_pull_request() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PULL_REQUEST_JSON)
        .create();

    let pr = client(&server)
        .pulls(&test_repo())
        .get(12)
        .expect("get should succeed");

    mock.assert();
    assert_eq!(pr.number, 12);
    assert_eq!(pr.head.git_ref, "feature/paging");
    assert_eq!(pr.base.git_ref, "master");
}

#[test]
fn files_deserializes_file_diffs() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/files";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PR_FILE_DIFF_JSON)
        .create();

    let files = client(&server)
        .pulls(&test_repo())
        .files(12)
        .expect("files should succeed");

    mock.assert();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].filename, "pom.xml");
    assert!(files[0].patch.as_ref().unwrap().contains("3.5.15"));
    assert!(files[1].patch.is_none());
}

#[test]
fn create_posts_form_title_head_base_body() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("title".into(), "New feature".into()),
            mockito::Matcher::UrlEncoded("head".into(), "feature/x".into()),
            mockito::Matcher::UrlEncoded("base".into(), "master".into()),
            mockito::Matcher::UrlEncoded("body".into(), "Please review".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PULL_REQUEST_JSON)
        .create();

    let pr = client(&server)
        .pulls(&test_repo())
        .create(&CreatePr {
            title: "New feature",
            head: "feature/x",
            base: "master",
            body: Some("Please review"),
        })
        .expect("create should succeed");

    mock.assert();
    assert_eq!(pr.number, 12);
}

#[test]
fn merge_puts_squash_and_close_related_issue_true() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/merge";

    let mock = server
        .mock("PUT", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("merge_method".into(), "squash".into()),
            mockito::Matcher::UrlEncoded("close_related_issue".into(), "true".into()),
        ]))
        .with_status(200)
        .create();

    client(&server)
        .pulls(&test_repo())
        .merge(12, MergeMethod::Squash, true)
        .expect("merge should succeed");

    mock.assert();
}

#[test]
fn merge_puts_merge_and_close_related_issue_false() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/merge";

    let mock = server
        .mock("PUT", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("merge_method".into(), "merge".into()),
            mockito::Matcher::UrlEncoded("close_related_issue".into(), "false".into()),
        ]))
        .with_status(200)
        .create();

    client(&server)
        .pulls(&test_repo())
        .merge(12, MergeMethod::Merge, false)
        .expect("merge should succeed");

    mock.assert();
}

#[test]
fn set_state_patches_form_state_closed() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12";

    let mock = server
        .mock("PATCH", api_path(path).as_str())
        .match_body(mockito::Matcher::UrlEncoded(
            "state".into(),
            "closed".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PULL_REQUEST_JSON)
        .create();

    let pr = client(&server)
        .pulls(&test_repo())
        .set_state(12, PrState::Closed)
        .expect("set_state should succeed");

    mock.assert();
    assert_eq!(pr.number, 12);
}

#[test]
fn comment_posts_form_body() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/comments";
    let response = r#"{"id":99,"body":"LGTM","html_url":"https://gitee.com/oschina/gitee-cli/pulls/12#note_99"}"#;

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::UrlEncoded("body".into(), "LGTM".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response)
        .create();

    let comment = client(&server)
        .pulls(&test_repo())
        .comment(12, "LGTM")
        .expect("comment should succeed");

    mock.assert();
    assert_eq!(comment.body, "LGTM");
}

#[test]
fn approve_force_true_sends_force_field() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/review";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::UrlEncoded("force".into(), "true".into()))
        .with_status(200)
        .create();

    client(&server)
        .pulls(&test_repo())
        .approve(12, true)
        .expect("approve should succeed");

    mock.assert();
}

#[test]
fn approve_force_false_sends_empty_form() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/review";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::Exact(String::new()))
        .with_status(200)
        .create();

    client(&server)
        .pulls(&test_repo())
        .approve(12, false)
        .expect("approve should succeed");

    mock.assert();
}

#[test]
fn link_get_then_patch_appends_linked_tag() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12";

    let get = server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":12,"title":"Link me","body":"Existing body","html_url":"https://gitee.com/x"}"#,
        )
        .create();

    let patch = server
        .mock("PATCH", api_path(path).as_str())
        .match_body(mockito::Matcher::Regex(r"Linked%3A\+%2342".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PULL_REQUEST_JSON)
        .create();

    let linked = client(&server)
        .pulls(&test_repo())
        .link(12, "#42")
        .expect("link should succeed");

    get.assert();
    patch.assert();
    assert!(linked);
}

#[test]
fn link_already_linked_skips_patch() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12";

    let get = server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":12,"title":"Done","body":"Already Linked: #42","html_url":"https://gitee.com/x"}"#,
        )
        .create();

    let patch = server
        .mock("PATCH", api_path(path).as_str())
        .expect(0)
        .create();

    let linked = client(&server)
        .pulls(&test_repo())
        .link(12, "#42")
        .expect("link should short-circuit");

    get.assert();
    patch.assert();
    assert!(!linked);
}
