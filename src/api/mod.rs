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

/// Outcome of an idempotent mutating call: did the resource change, or was it
/// already in the requested state? The wrapped object is the current state of
/// the resource either way, so callers can render it consistently.
#[derive(Debug, Clone)]
pub enum StateChange<T> {
    Changed(T),
    Already(T),
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
