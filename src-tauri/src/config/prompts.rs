use super::Config;
use crate::error::AppResult;

/// Built-in agent jobs, each backed by `prompts/<name>.md`.
#[derive(Debug, Clone, Copy)]
pub enum Prompt {
    Chat,
    DraftSchedule,
    ResolveUrgent,
    RunSchedule,
    Continue,
    HandOver,
    FixCi,
    FixConflicts,
    CiDone,
    Answer,
    Message,
    Workspace,
    IssueWorkspace,
    NewWorkspace,
    NoWorkspace,
    Queue,
}

impl Prompt {
    const fn file(self) -> &'static str {
        match self {
            Self::Chat => "chat.md",
            Self::DraftSchedule => "draft-schedule.md",
            Self::ResolveUrgent => "resolve-urgent.md",
            Self::RunSchedule => "run-schedule.md",
            Self::Continue => "continue.md",
            Self::HandOver => "hand-over.md",
            Self::FixCi => "fix-ci.md",
            Self::FixConflicts => "fix-conflicts.md",
            Self::CiDone => "ci-done.md",
            Self::Answer => "answer.md",
            Self::Message => "message.md",
            Self::Workspace => "workspace.md",
            Self::IssueWorkspace => "issue-workspace.md",
            Self::NewWorkspace => "new-workspace.md",
            Self::NoWorkspace => "no-workspace.md",
            Self::Queue => "queue.md",
        }
    }
}

/// Loads a prompt and fills its `{{key}}` placeholders.
pub fn render(config: &Config, prompt: Prompt, vars: &[(&str, &str)]) -> AppResult<String> {
    let template = std::fs::read_to_string(config.root().join("prompts").join(prompt.file()))?;
    Ok(fill(&template, vars))
}

fn fill(template: &str, vars: &[(&str, &str)]) -> String {
    vars.iter().fold(template.to_owned(), |text, (key, value)| {
        text.replace(&format!("{{{{{key}}}}}"), value)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_placeholders() {
        assert_eq!(
            fill("Hi {{name}}, {{name}}!", &[("name", "Oscar")]),
            "Hi Oscar, Oscar!"
        );
    }
}
