use gitee_cli_rs::api::client::Client;
use gitee_cli_rs::api::milestones::{CreateMilestone, EditMilestone, MilestoneFilter};
use gitee_cli_rs::repo::Repo;

const MILESTONE_JSON: &str = include_str!("fixtures/milestone.json");

fn client(server: &mockito::ServerGuard) -> Client {
    Client::new(format!("{}/api/v5", server.url()), "fake-token".into())
}

fn api_path(path: &str) -> String {
    format!("/api/v5{path}")
}

fn test_repo() -> Repo {
    Repo {
        owner: "oschina".into(),
        name: "gitee-cli".into(),
    }
}

#[test]
fn list_forwards_state_param() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/milestones";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!("[{MILESTONE_JSON}]"))
        .create();

    let items = client(&server)
        .milestones(&test_repo())
        .list(&MilestoneFilter {
            state: Some("open"),
            limit: 30,
        })
        .expect("list should succeed");

    mock.assert();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title, "v1.0");
}

#[test]
fn get_by_number() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/milestones/226762";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(MILESTONE_JSON)
        .create();

    let milestone = client(&server)
        .milestones(&test_repo())
        .get(226762)
        .expect("get should succeed");

    mock.assert();
    assert_eq!(milestone.number, 226762);
    assert_eq!(milestone.open_issues, Some(2));
}

#[test]
fn create_posts_form_fields() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/milestones";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("title".into(), "v1.0".into()),
            mockito::Matcher::UrlEncoded("due_on".into(), "2026-12-31".into()),
            mockito::Matcher::UrlEncoded("description".into(), "Ship it".into()),
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(MILESTONE_JSON)
        .create();

    client(&server)
        .milestones(&test_repo())
        .create(&CreateMilestone {
            title: "v1.0",
            due_on: "2026-12-31",
            description: Some("Ship it"),
            state: Some("open"),
        })
        .expect("create should succeed");

    mock.assert();
}

#[test]
fn edit_gets_first_then_patches_with_echoed_title_and_due_on() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/milestones/226762";
    let patch_path = "/repos/oschina/gitee-cli/milestones/226762";

    let get = server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(MILESTONE_JSON)
        .create();

    let patch = server
        .mock("PATCH", api_path(patch_path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("title".into(), "v1.0".into()),
            mockito::Matcher::UrlEncoded("due_on".into(), "2026-12-31".into()),
            mockito::Matcher::UrlEncoded("description".into(), "Updated".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(MILESTONE_JSON)
        .create();

    client(&server)
        .milestones(&test_repo())
        .edit(
            226762,
            &EditMilestone {
                description: Some("Updated"),
                ..Default::default()
            },
        )
        .expect("edit should succeed");

    get.assert();
    patch.assert();
}

#[test]
fn edit_title_uses_new_title_and_echoes_due_on() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/milestones/226762";
    let patch_path = "/repos/oschina/gitee-cli/milestones/226762";

    server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(MILESTONE_JSON)
        .create();

    let patch = server
        .mock("PATCH", api_path(patch_path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("title".into(), "v1.1".into()),
            mockito::Matcher::UrlEncoded("due_on".into(), "2026-12-31".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(MILESTONE_JSON)
        .create();

    client(&server)
        .milestones(&test_repo())
        .edit(
            226762,
            &EditMilestone {
                title: Some("v1.1"),
                ..Default::default()
            },
        )
        .expect("edit should succeed");

    patch.assert();
}
