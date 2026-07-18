use super::client::Client;
use crate::error::Result;
use crate::models::Webhook;
use crate::repo::Repo;

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
    pub pull_requests_events: bool,
    pub note_events: bool,
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
                "pull_requests_events",
                Client::bool_str(req.pull_requests_events),
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
