use std::collections::BTreeMap;

pub type SessionId = String;

#[derive(Debug, Clone)]
pub struct Session {
    pub user: String,
    pub active: bool,
}

#[derive(Debug, Default)]
pub struct State {
    sessions: BTreeMap<SessionId, Session>,
    next_id: u64,
}

impl State {
    pub fn new() -> Self {
        State {
            sessions: BTreeMap::new(),
            next_id: 0,
        }
    }

    pub fn create(&mut self, user: String) -> SessionId {
        let id = format!("s{}", self.next_id);
        self.next_id += 1;

        self.sessions.insert(
            id.clone(),
            Session {
                user,
                active: true,
            },
        );

        id
    }

    pub fn revoke(&mut self, session_id: &str) {
        if let Some(s) = self.sessions.get_mut(session_id) {
            s.active = false;
        }
    }

    pub fn validate(&self, session_id: &str) -> bool {
        self.sessions
            .get(session_id)
            .map(|s| s.active)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_marks_session_active() {
        let mut state = State::new();
        let session_id = state.create("alice".to_string());

        assert!(state.validate(&session_id));
        assert_eq!(session_id, "s0");
    }

    #[test]
    fn revoke_disables_session() {
        let mut state = State::new();
        let session_id = state.create("alice".to_string());

        state.revoke(&session_id);

        assert!(!state.validate(&session_id));
    }

    #[test]
    fn validation_isolated_per_session() {
        let mut state = State::new();
        let first = state.create("alice".to_string());
        let second = state.create("bob".to_string());

        state.revoke(&first);

        assert!(!state.validate(&first));
        assert!(state.validate(&second));
    }
}
