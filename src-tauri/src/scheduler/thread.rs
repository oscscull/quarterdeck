//! Which session an item's work continues in: the one holding it, or a new one.

use crate::domain::session::SessionRef;
use crate::workspace::Workspace;

/// Whether a run continues a session or starts one Quarterdeck can come back to.
#[derive(Debug, Clone)]
pub enum Thread {
    Start(SessionRef),
    Resume(SessionRef),
}

impl Thread {
    pub const fn session(&self) -> &SessionRef {
        match self {
            Self::Start(s) | Self::Resume(s) => s,
        }
    }
}

/// A new session in Quarterdeck's own folder, for repos with no project here.
pub fn fresh_session() -> SessionRef {
    SessionRef {
        id: uuid::Uuid::new_v4().to_string(),
        cwd: None,
    }
}

/// Where work on an item continues: the known session if it ran in the item's worktree (or there
/// is none), else a new session in the worktree. None with neither.
pub fn thread_for(known: Option<SessionRef>, ws: Option<&Workspace>) -> Option<Thread> {
    match (known, ws) {
        (Some(s), Some(ws)) if ws.holds(&s) => Some(Thread::Resume(s)),
        (_, Some(ws)) => Some(Thread::Start(ws.new_session())),
        (Some(s), None) => Some(Thread::Resume(s)),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Kind;

    fn session(cwd: Option<&str>) -> SessionRef {
        SessionRef {
            id: "s".to_owned(),
            cwd: cwd.map(str::to_owned),
        }
    }

    #[test]
    fn only_items_with_no_session_and_no_workspace_have_nobody_to_tell() {
        let ws = Workspace {
            project: "/code/app".into(),
            repo: "me/app".to_owned(),
            kind: Kind::Issue(3),
            dir: "/wt/app-3".into(),
            branch: Some("issue-3".to_owned()),
        };
        assert!(thread_for(None, None).is_none());
        assert!(matches!(
            thread_for(Some(session(None)), None),
            Some(Thread::Resume(_))
        ));
        assert!(matches!(
            thread_for(None, Some(&ws)),
            Some(Thread::Start(_))
        ));
        assert!(matches!(
            thread_for(Some(session(Some("/wt/app-3"))), Some(&ws)),
            Some(Thread::Resume(_))
        ));
    }
}
