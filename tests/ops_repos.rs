use gitee_cli_rs::api::client::Client;

const REPO_LIST_JSON: &str = include_str!("fixtures/repo_list.json");
const REPO_JSON: &str = r#"{
  "id": 1001,
  "name": "gitee-cli",
  "full_name": "oschina/gitee-cli",
  "human_name": "oschina/gitee-cli",
  "description": "A gh-like command-line client for Gitee",
  "html_url": "https://gitee.com/oschina/gitee-cli",
  "ssh_url": "git@gitee.com:oschina/gitee-cli.git",
  "clone_url": "https://gitee.com/oschina/gitee-cli.git",
  "default_branch": "master",
  "private": false,
  "stargazers_count": 128,
  "fork_count": 24,
  "open_issues_count": 6
}"#;

fn client(server: &mockito::ServerGuard) -> Client {
    Client::new(format!("{}/api/v5", server.url()), "fake-token".into())
}

fn api_path(path: &str) -> String {
    format!("/api/v5{path}")
}

#[test]
fn list_mine_hits_user_repos() {
    let mut server = mockito::Server::new();
    let path = "/user/repos";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(REPO_LIST_JSON)
        .create();

    let items = client(&server)
        .repos()
        .list_mine(10)
        .expect("list_mine should succeed");

    mock.assert();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].full_name, "oschina/gitee-cli");
}

#[test]
fn list_user_hits_users_repos_path() {
    let mut server = mockito::Server::new();
    let path = "/users/oschina/repos";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(REPO_LIST_JSON)
        .create();

    let items = client(&server)
        .repos()
        .list_user("oschina", 10)
        .expect("list_user should succeed");

    mock.assert();
    assert_eq!(items[1].full_name, "oschina/docs");
    assert_eq!(items[1].private, Some(true));
}

#[test]
fn get_hits_repos_owner_name() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(REPO_JSON)
        .create();

    let repo = client(&server)
        .repos()
        .get("oschina", "gitee-cli")
        .expect("get should succeed");

    mock.assert();
    assert_eq!(repo.full_name, "oschina/gitee-cli");
    assert_eq!(repo.stargazers_count, Some(128));
}

#[test]
fn fork_posts_repos_owner_name_forks() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/forks";
    let forked = r#"{
        "id": 3003,
        "name": "gitee-cli",
        "full_name": "dev1/gitee-cli",
        "human_name": "dev1/gitee-cli",
        "html_url": "https://gitee.com/dev1/gitee-cli",
        "private": false,
        "stargazers_count": 0,
        "parent": { "full_name": "oschina/gitee-cli", "html_url": "https://gitee.com/oschina/gitee-cli" }
    }"#;

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::Exact(String::new()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(forked)
        .create();

    let repo = client(&server)
        .repos()
        .fork("oschina", "gitee-cli")
        .expect("fork should succeed");

    mock.assert();
    assert_eq!(repo.full_name, "dev1/gitee-cli");
    assert!(repo.parent.is_some());
}
