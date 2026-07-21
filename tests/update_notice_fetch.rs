use std::time::Duration;

use gitee_cli_rs::update_notice::{fetch_latest, ReleaseInfo};

const LATEST_JSON: &str = r#"{
  "tag_name": "v0.2.0",
  "html_url": "https://github.com/kipyin/gitee-cli/releases/tag/v0.2.0",
  "published_at": "2026-01-15T12:00:00Z"
}"#;

#[test]
fn fetch_latest_returns_version_and_url_on_200() {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/repos/kipyin/gitee-cli/releases/latest")
        .match_header("accept", "application/vnd.github+json")
        .match_header("x-github-api-version", "2022-11-28")
        .match_header("user-agent", mockito::Matcher::Regex("gitee-cli".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(LATEST_JSON)
        .create();

    let info = fetch_latest(&server.url(), Duration::from_secs(2))
        .expect("200 should yield ReleaseInfo");

    mock.assert();
    assert_eq!(
        info,
        ReleaseInfo {
            version: "0.2.0".into(),
            url: "https://github.com/kipyin/gitee-cli/releases/tag/v0.2.0".into(),
        }
    );
}

#[test]
fn fetch_latest_returns_none_on_non_200() {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/repos/kipyin/gitee-cli/releases/latest")
        .with_status(500)
        .with_body("oops")
        .create();

    assert!(fetch_latest(&server.url(), Duration::from_secs(2)).is_none());
    mock.assert();
}

#[test]
fn fetch_latest_returns_none_on_decode_error() {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/repos/kipyin/gitee-cli/releases/latest")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{not-json")
        .create();

    assert!(fetch_latest(&server.url(), Duration::from_secs(2)).is_none());
    mock.assert();
}

#[test]
fn fetch_latest_returns_none_on_timeout() {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/repos/kipyin/gitee-cli/releases/latest")
        .with_chunked_body(|w| {
            std::thread::sleep(Duration::from_secs(1));
            w.write_all(LATEST_JSON.as_bytes())
        })
        .create();

    let timeout = Duration::from_millis(100);
    assert!(
        fetch_latest(&server.url(), timeout).is_none(),
        "slow response should time out silently"
    );
    mock.assert();
}
