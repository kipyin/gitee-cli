use super::client::Client;
use crate::error::Result;
use crate::models::Milestone;
use crate::repo::Repo;

pub struct Milestones<'a> {
    client: &'a Client,
    repo: &'a Repo,
}

pub struct MilestoneFilter<'a> {
    pub state: Option<&'a str>,
    pub limit: usize,
}

pub struct CreateMilestone<'a> {
    pub title: &'a str,
    /// Gitee requires `due_on` on create (400 without it). Accepts `YYYY-MM-DD`.
    pub due_on: &'a str,
    pub description: Option<&'a str>,
    pub state: Option<&'a str>,
}

#[derive(Default)]
pub struct EditMilestone<'a> {
    pub title: Option<&'a str>,
    pub due_on: Option<&'a str>,
    pub description: Option<&'a str>,
    pub state: Option<&'a str>,
}

impl Milestones<'_> {
    pub(crate) fn new<'a>(client: &'a Client, repo: &'a Repo) -> Milestones<'a> {
        Milestones { client, repo }
    }

    pub fn list(&self, filter: &MilestoneFilter<'_>) -> Result<Vec<Milestone>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let path = format!("/repos/{o}/{r}/milestones");
        let mut q: Vec<(&str, &str)> = Vec::new();
        if let Some(s) = filter.state {
            q.push(("state", s));
        }
        self.client.get_paged(&path, &q, filter.limit)
    }

    pub fn get(&self, number: i64) -> Result<Milestone> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        self.client
            .get(&format!("/repos/{o}/{r}/milestones/{number}"), &[])
    }

    pub fn create(&self, req: &CreateMilestone<'_>) -> Result<Milestone> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let mut f: Vec<(&str, String)> = vec![
            ("title", req.title.to_string()),
            ("due_on", req.due_on.to_string()),
        ];
        if let Some(d) = req.description {
            f.push(("description", d.to_string()));
        }
        if let Some(s) = req.state {
            f.push(("state", s.to_string()));
        }
        let form = Client::str_refs(&f);
        self.client
            .post(&format!("/repos/{o}/{r}/milestones"), &form)
    }

    /// Gitee quirk: PATCH requires both `title` and `due_on` every time (400 if either
    /// is missing). Fetch the current milestone and echo unset fields, like
    /// `Issues::edit` echoing the current title.
    pub fn edit(&self, number: i64, req: &EditMilestone<'_>) -> Result<Milestone> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let cur = self.get(number)?;
        let title = req.title.unwrap_or(&cur.title);
        let due_on = req
            .due_on
            .or(cur.due_on.as_deref())
            .unwrap_or_default();
        let mut f: Vec<(&str, String)> = vec![
            ("title", title.to_string()),
            ("due_on", due_on.to_string()),
        ];
        if let Some(d) = req.description {
            f.push(("description", d.to_string()));
        }
        if let Some(s) = req.state {
            f.push(("state", s.to_string()));
        }
        let form = Client::str_refs(&f);
        self.client
            .patch(&format!("/repos/{o}/{r}/milestones/{number}"), &form)
    }
}
