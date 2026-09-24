pub mod client;
pub mod collaborators;
pub mod gists;
pub mod issues;
pub mod labels;
pub mod milestones;
pub mod pulls;
pub mod releases;
pub mod repos;
pub mod search;
pub mod users;
pub mod webhooks;

use crate::models::{Comment, PrComment};
use std::collections::HashSet;

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
    pub fn was_changed(&self) -> bool {
        matches!(self, StateChange::Changed(_))
    }
}

/// Names from `requested` that are not in `present`, in request order.
/// A repeated name is kept once, at its first occurrence.
pub(crate) fn missing_names<'a>(requested: &[&'a str], present: &HashSet<&str>) -> Vec<&'a str> {
    select_names(requested, present, false)
}

/// Names from `requested` that are already in `present`, in request order.
/// A repeated name is kept once, at its first occurrence.
pub(crate) fn present_names<'a>(requested: &[&'a str], present: &HashSet<&str>) -> Vec<&'a str> {
    select_names(requested, present, true)
}

fn select_names<'a>(
    requested: &[&'a str],
    present: &HashSet<&str>,
    want_present: bool,
) -> Vec<&'a str> {
    let mut seen = HashSet::new();
    requested
        .iter()
        .copied()
        .filter(|name| present.contains(name) == want_present && seen.insert(*name))
        .collect()
}

#[cfg(test)]
mod name_selection_tests {
    use super::{missing_names, present_names};
    use std::collections::HashSet;

    fn set<'a>(names: &[&'a str]) -> HashSet<&'a str> {
        names.iter().copied().collect()
    }

    #[test]
    fn missing_names_keeps_order_and_drops_duplicates() {
        let present = set(&["bug"]);
        assert_eq!(
            missing_names(&["ui", "bug", "ui", "docs"], &present),
            ["ui", "docs"]
        );
    }

    #[test]
    fn present_names_keeps_members_once() {
        let present = set(&["bug", "ui"]);
        assert_eq!(
            present_names(&["missing", "bug", "bug", "ui"], &present),
            ["bug", "ui"]
        );
    }

    #[test]
    fn empty_request_selects_nothing() {
        let present = set(&["bug"]);
        assert!(missing_names(&[], &present).is_empty());
        assert!(present_names(&[], &present).is_empty());
    }
}
