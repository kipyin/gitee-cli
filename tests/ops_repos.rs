use gitee_cli_rs::api::client::Client;
use gitee_cli_rs::api::repos::{CreateRepo, EditRepo};

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
#[test]
fn create_posts_user_repos_with_private_true() {
    let mut server = mockito::Server::new();
    let path = "/user/repos";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("name".into(), "smoke-repo".into()),
            mockito::Matcher::UrlEncoded("description".into(), "A test repo".into()),
            mockito::Matcher::UrlEncoded("homepage".into(), "https://example.com".into()),
            mockito::Matcher::UrlEncoded("gitignore_template".into(), "Rust".into()),
            mockito::Matcher::UrlEncoded("license_template".into(), "MIT".into()),
            mockito::Matcher::UrlEncoded("private".into(), "true".into()),
        ]))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(REPO_JSON)
        .create();

    let repo = client(&server)
        .repos()
        .create(&CreateRepo {
            name: "smoke-repo",
            org: None,
            description: Some("A test repo"),
            homepage: Some("https://example.com"),
            gitignore_template: Some("Rust"),
            license_template: Some("MIT"),
            private: true,
        })
        .expect("create should succeed");

    mock.assert();
    assert_eq!(repo.full_name, "oschina/gitee-cli");
}

#[test]
fn create_org_posts_orgs_repos_path() {
    let mut server = mockito::Server::new();
    let path = "/orgs/acme/repos";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("name".into(), "team-repo".into()),
            mockito::Matcher::UrlEncoded("private".into(), "false".into()),
        ]))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(REPO_JSON)
        .create();

    client(&server)
        .repos()
        .create(&CreateRepo {
            name: "team-repo",
            org: Some("acme"),
            description: None,
            homepage: None,
            gitignore_template: None,
            license_template: None,
            private: false,
        })
        .expect("create should succeed");

    mock.assert();
}

#[test]
fn edit_sends_current_name_and_only_provided_flags() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli";

    let mock = server
        .mock("PATCH", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("name".into(), "gitee-cli".into()),
            mockito::Matcher::UrlEncoded("description".into(), "updated".into()),
            mockito::Matcher::UrlEncoded("private".into(), "true".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(REPO_JSON)
        .create();

    client(&server)
        .repos()
        .edit(
            "oschina",
            "gitee-cli",
            &EditRepo {
                name: "gitee-cli",
                description: Some("updated"),
                homepage: None,
                private: Some(true),
                default_branch: None,
            },
        )
        .expect("edit should succeed");

    mock.assert();
}

#[test]
fn rename_sends_name_and_path() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli";

    let mock = server
        .mock("PATCH", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("name".into(), "gitee-cli".into()),
            mockito::Matcher::UrlEncoded("path".into(), "gitee-cli-renamed".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(REPO_JSON)
        .create();

    client(&server)
        .repos()
        .rename("oschina", "gitee-cli", "gitee-cli", "gitee-cli-renamed")
        .expect("rename should succeed");

    mock.assert();
}

#[test]
fn delete_hits_repos_owner_name() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli";

    let mock = server
        .mock("DELETE", api_path(path).as_str())
        .with_status(204)
        .create();

    client(&server)
        .repos()
        .delete("oschina", "gitee-cli")
        .expect("delete should succeed");

    mock.assert();
}

