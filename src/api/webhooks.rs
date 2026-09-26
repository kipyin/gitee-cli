use super::client::Client;
use crate::cli::join_flags;
use crate::error::{GiteeError, Result};
use crate::models::Webhook;
use crate::repo::Repo;

/// `(cli token, form field)`. One row per accepted `--events` token.
/// A row whose token differs from its field is an alias: `pull_requests_events`
/// turns on `merge_requests_events` and does not add a form field.
macro_rules! event_specs {
    ($(($cli:literal, $field:literal)),+ $(,)?) => {
        const EVENT_SPECS: &[(&str, &str)] = &[$(($cli, $field)),+];
        /// Accepted `--events` tokens, in spec order.
        pub const EVENT_FLAGS: &[&str] = &[$($cli),+];
    };
}

event_specs! {
    ("push_events", "push_events"),
    ("tag_push_events", "tag_push_events"),
    ("issues_events", "issues_events"),
    ("merge_requests_events", "merge_requests_events"),
    ("pull_requests_events", "merge_requests_events"),
    ("note_events", "note_events"),
}

/// Form fields in spec order. Alias rows (token ≠ field) are omitted.
fn form_fields() -> [&'static str; 5] {
    let fields: Vec<&'static str> = EVENT_SPECS
        .iter()
        .filter(|(cli, field)| cli == field)
        .map(|(_, field)| *field)
        .collect();
    fields
        .try_into()
        .expect("canonical webhook events are the rows where the CLI token is the form field")
}

fn field_on(events: &[String], field: &str) -> bool {
    events.iter().any(|ev| {
        EVENT_SPECS
            .iter()
            .any(|(cli, api)| *cli == ev.as_str() && *api == field)
    })
}

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
/// Tuple order is `form_fields()`: push, tag push, issues, merge requests, note.
pub fn event_bools(events: &[String]) -> (bool, bool, bool, bool, bool) {
    let [push, tag, issues, merge, note] = form_fields().map(|field| field_on(events, field));
    (push, tag, issues, merge, note)
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
        // Same order as `event_bools` / `form_fields()`.
        let enabled = [
            req.push_events,
            req.tag_push_events,
            req.issues_events,
            req.merge_requests_events,
            req.note_events,
        ];
        let mut form: Vec<(&str, &str)> = vec![("url", req.url)];
        for (field, on) in form_fields().into_iter().zip(enabled) {
            form.push((field, Client::bool_str(on)));
        }
        if let Some(password) = req.password {
            form.push(("password", password));
        }
        self.client.post(&format!("/repos/{o}/{r}/hooks"), &form)
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        let (o, r) = (&self.repo.owner, &self.repo.name);
        self.client.delete_ok(&format!("/repos/{o}/{r}/hooks/{id}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_events_defaults_to_push() {
        assert_eq!(parse_events(&[]).unwrap(), vec!["push_events"]);
        assert_eq!(parse_events(&[" ".into()]).unwrap(), vec!["push_events"]);
        assert_eq!(parse_events(&[",".into()]).unwrap(), vec!["push_events"]);
    }

    #[test]
    fn parse_events_splits_commas_and_skips_blanks() {
        let events = parse_events(&[" push_events, note_events ".into(), " ".into()]).unwrap();
        assert_eq!(events, vec!["push_events", "note_events"]);
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
    fn each_event_enables_only_its_form_field() {
        let cases = [
            ("push_events", [true, false, false, false, false]),
            ("tag_push_events", [false, true, false, false, false]),
            ("issues_events", [false, false, true, false, false]),
            ("merge_requests_events", [false, false, false, true, false]),
            ("pull_requests_events", [false, false, false, true, false]),
            ("note_events", [false, false, false, false, true]),
        ];
        for (name, expect) in cases {
            let (push, tag, issues, merge, note) = event_bools(&[name.into()]);
            assert_eq!([push, tag, issues, merge, note], expect, "{name}");
        }
    }

    #[test]
    fn parse_events_rejects_unknown() {
        assert!(parse_events(&["bogus".into()]).is_err());
    }
}
