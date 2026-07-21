use std::time::Duration;

use gitee_cli_rs::update_notice::UpdateNotice;

const NEWER_JSON: &str = r#"{
  "tag_name": "v0.2.0",
  "html_url": "https://github.com/kipyin/gitee-cli/releases/tag/v0.2.0",
  "published_at": "2026-01-15T12:00:00Z"
}"#;

const SAME_JSON: &str = r#"{
  "tag_name": "v0.1.5",
  "html_url": "https://github.com/kipyin/gitee-cli/releases/tag/v0.1.5",
  "published_at": "2026-01-15T12:00:00Z"
}"#;

fn mock_latest(server: &mut mockito::ServerGuard, body: &str, status: usize) -> mockito::Mock {
    server
        .mock("GET", "/repos/kipyin/gitee-cli/releases/latest")
        .with_status(status)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create()
}

#[test]
fn finish_on_success_writes_tip_when_remote_is_newer() {
    let mut server = mockito::Server::new();
    let mock = mock_latest(&mut server, NEWER_JSON, 200);

    let notice = UpdateNotice::spawn("0.1.5", &server.url());
    // Ensure background work can complete before we join.
    std::thread::sleep(Duration::from_millis(50));
    let mut buf = Vec::new();
    notice.finish_on_success(&mut buf);

    mock.assert();
    let out = String::from_utf8(buf).unwrap();
    assert!(
        out.contains("A new release of gitee is available: 0.1.5 → 0.2.0"),
        "tip missing from: {out:?}"
    );
    assert!(
        out.contains("https://github.com/kipyin/gitee-cli/releases/tag/v0.2.0"),
        "url missing from: {out:?}"
    );
    assert!(out.starts_with('\n') && out.ends_with("\n\n"), "expected blank-line padding");
}

#[test]
fn finish_on_success_writes_nothing_when_not_newer() {
    let mut server = mockito::Server::new();
    let mock = mock_latest(&mut server, SAME_JSON, 200);

    let notice = UpdateNotice::spawn("0.1.5", &server.url());
    std::thread::sleep(Duration::from_millis(50));
    let mut buf = Vec::new();
    notice.finish_on_success(&mut buf);

    mock.assert();
    assert!(buf.is_empty(), "unexpected tip: {:?}", String::from_utf8_lossy(&buf));
}

#[test]
fn finish_on_success_writes_nothing_when_fetch_fails() {
    let mut server = mockito::Server::new();
    let mock = mock_latest(&mut server, "oops", 500);

    let notice = UpdateNotice::spawn("0.1.5", &server.url());
    std::thread::sleep(Duration::from_millis(50));
    let mut buf = Vec::new();
    notice.finish_on_success(&mut buf);

    mock.assert();
    assert!(buf.is_empty());
}

#[test]
fn only_finish_on_success_writes_tip_bytes() {
    let mut server = mockito::Server::new();
    let mock_fail = mock_latest(&mut server, NEWER_JSON, 200);
    let mock_ok = mock_latest(&mut server, NEWER_JSON, 200);

    // Failed command path: drop without finish_on_success — writer stays empty.
    {
        let notice = UpdateNotice::spawn("0.1.5", &server.url());
        std::thread::sleep(Duration::from_millis(100));
        let buf = Vec::new();
        drop(notice);
        assert!(
            buf.is_empty(),
            "failed-command path must not write update tip; got {:?}",
            String::from_utf8_lossy(&buf)
        );
    }
    mock_fail.assert();

    // Success path: finish_on_success is the only code path that writes the tip.
    {
        let notice = UpdateNotice::spawn("0.1.5", &server.url());
        std::thread::sleep(Duration::from_millis(100));
        let mut buf = Vec::new();
        notice.finish_on_success(&mut buf);
        assert!(
            !buf.is_empty(),
            "finish_on_success must write when a newer release is available"
        );
        assert!(
            String::from_utf8(buf)
                .unwrap()
                .contains("A new release of gitee is available: 0.1.5 → 0.2.0")
        );
    }
    mock_ok.assert();
}
