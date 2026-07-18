use super::client::Client;
use crate::error::{GiteeError, Result};
use crate::models::Webhook;
use crate::repo::Repo;

pub const EVENT_FLAGS: &[&str] = &[
    "push_events",
    "tag_push_events",
    "issues_events",
    "merge_requests_events",
    "pull_requests_events", // alias → merge_requests_events
    "note_events",
];

pub struct Webhooks<'a> {
    client: &'a Client,
    repo: &'a Repo,
}

pub struct CreateWebhook<'a> {
    pub url: &'a str,
    pub password: Option<&'a str>,
    pub push_events: bool,
    pub tag_push_events: bool,
    pub issues_events: bool,
    pub merge_requests_events: bool,
    pub note_events: bool,
}

/// Parse comma-separated webhook event flags from CLI `--events` values.
/// Defaults to `push_events` when nothing is specified.
pub fn parse_events(raw: &[String]) -> Result<Vec<String>> {
    let joined = join_flags(raw).unwrap_or_default();
    if joined.is_empty() {
        return Ok(vec!["push_events".into()]);
    }
    let mut out = Vec::new();
    for ev in joined.split(',') {
        let ev = ev.trim();
        if !EVENT_FLAGS.contains(&ev) {
            return Err(GiteeError::Usage(format!(
                "unknown webhook event '{ev}'; expected one of {}",
                EVENT_FLAGS.join(", ")
            )));
        }
        if !out.iter().any(|e| e == ev) {
            out.push(ev.to_string());
        }
    }
    Ok(out)
}

/// Map parsed event names to the bool fields Gitee expects on create.
pub fn event_bools(events: &[String]) -> (bool, bool, bool, bool, bool) {
    (
        events.iter().any(|e| e == "push_events"),
        events.iter().any(|e| e == "tag_push_events"),
        events.iter().any(|e| e == "issues_events"),
        events.iter().any(|e| {
            e == "merge_requests_events" || e == "pull_requests_events"
        }),
        events.iter().any(|e| e == "note_events"),
    )
}

fn join_flags(values: &[String]) -> Option<String> {
    if values.is_empty() {
        return None;
    }
    Some(
        values
            .iter()
            .flat_map(|v| v.split(','))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(","),
    )
}

impl Webhooks<'_> {
    pub(crate) fn new<'a>(client: &'a Client, repo: &'a Repo) -> Webhooks<'a> {
        Webhooks { client, repo }
    }

    pub fn list(&self, limit: usize) -> Result<Vec<Webhook>> {
        let (o, r) = (&self.repo.owner, &self.repo.name);
        self.client
            .get_paged(&format!("/repos/{o}/{r}/hooks"), &[], limit)
    }

    pub fn create(&self, req: &CreateWebhook<'_>) -> Result<Webhook> {
        let (o, r) = (&self.repo.owner, &self.repo.name);
        let mut form: Vec<(&str, &str)> = vec![
            ("url", req.url),
            ("push_events", Client::bool_str(req.push_events)),
            ("tag_push_events", Client::bool_str(req.tag_push_events)),
            ("issues_events", Client::bool_str(req.issues_events)),
            (
                "merge_requests_events",
                Client::bool_str(req.merge_requests_events),
            ),
            ("note_events", Client::bool_str(req.note_events)),
        ];
        if let Some(password) = req.password {
            form.push(("password", password));
        }
        self.client
            .post(&format!("/repos/{o}/{r}/hooks"), &form)
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        let (o, r) = (&self.repo.owner, &self.repo.name);
        self.client
            .delete_ok(&format!("/repos/{o}/{r}/hooks/{id}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_events_defaults_to_push() {
        assert_eq!(parse_events(&[]).unwrap(), vec!["push_events"]);
    }

    #[test]
    fn parse_events_accepts_pull_requests_alias() {
        let events = parse_events(&["pull_requests_events".into()]).unwrap();
        assert_eq!(events, vec!["pull_requests_events"]);
        let (push, tag, issues, merge, note) = event_bools(&events);
        assert!(!push);
        assert!(!tag);
        assert!(!issues);
        assert!(merge);
        assert!(!note);
    }

    #[test]
    fn parse_events_rejects_unknown() {
        assert!(parse_events(&["bogus".into()]).is_err());
    }
}
