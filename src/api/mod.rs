pub mod client;
pub mod collaborators;
pub mod gists;
pub mod issues;
pub mod labels;
pub mod pulls;
pub mod milestones;
pub mod releases;
pub mod repos;
pub mod search;
pub mod users;
pub mod webhooks;

use crate::models::{Comment, PrComment};

/// Outcome of an idempotent mutating call: did the resource change, or was it
/// already in the requested state? The wrapped object is the current state of
/// the resource either way, so callers can render it consistently.
#[derive(Debug, Clone)]
pub enum StateChange<T> {
    Changed(T),
    Already(T),
}

/// Minimal view of a comment for `--last` resolution (author + created_at).
pub trait AuthoredComment {
    fn author_login(&self) -> Option<&str>;
    fn created_at_str(&self) -> Option<&str>;
}

impl AuthoredComment for Comment {
    fn author_login(&self) -> Option<&str> {
        self.user.as_ref().map(|u| u.login.as_str())
    }
    fn created_at_str(&self) -> Option<&str> {
        self.created_at.as_deref()
    }
}

impl AuthoredComment for PrComment {
    fn author_login(&self) -> Option<&str> {
        self.user.as_ref().map(|u| u.login.as_str())
    }
    fn created_at_str(&self) -> Option<&str> {
        self.created_at.as_deref()
    }
}

/// Pick the comment by `login` with the greatest `created_at` string.
/// ISO-8601 timestamps from Gitee sort lexicographically.
pub fn resolve_latest_comment<'a, T: AuthoredComment>(
    comments: &'a [T],
    login: &str,
) -> Option<&'a T> {
    comments
        .iter()
        .filter(|c| c.author_login() == Some(login))
        .max_by_key(|c| c.created_at_str().unwrap_or(""))
}

impl<T> StateChange<T> {
    pub fn into_inner(self) -> T {
        match self {
            StateChange::Changed(t) | StateChange::Already(t) => t,
        }
    }

    pub fn was_changed(&self) -> bool {
        matches!(self, StateChange::Changed(_))
    }
}
