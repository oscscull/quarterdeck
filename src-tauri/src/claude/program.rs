//! Which Claude Code to run, and the model every run is locked to.

use tokio::process::Command;

/// Claude Code's override for the model its subagents use, which otherwise may differ per agent.
const SUBAGENT_MODEL: &str = "CLAUDE_CODE_SUBAGENT_MODEL";

/// How to start Claude Code for a headless run.
#[derive(Debug, Clone)]
pub struct Program {
    pub command: String,
    /// The model every run uses, subagents included. Empty leaves it to Claude Code.
    pub model: String,
}

impl Program {
    /// Pins `cmd` to the model, so neither Claude Code's defaults nor a subagent can pick another.
    pub fn lock(&self, cmd: &mut Command) {
        if self.model.is_empty() {
            return;
        }
        cmd.arg("--model")
            .arg(&self.model)
            .env(SUBAGENT_MODEL, &self.model);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(model: &str) -> Vec<String> {
        let program = Program {
            command: "claude".to_owned(),
            model: model.to_owned(),
        };
        let mut cmd = Command::new("claude");
        program.lock(&mut cmd);
        let std = cmd.as_std();
        let mut seen: Vec<String> = std
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        seen.extend(
            std.get_envs()
                .filter_map(|(k, v)| Some(format!("{}={}", k.to_str()?, v?.to_str()?))),
        );
        seen
    }

    #[test]
    fn a_model_locks_the_run_and_its_subagents() {
        assert_eq!(
            args("claude-opus-5-5"),
            [
                "--model",
                "claude-opus-5-5",
                "CLAUDE_CODE_SUBAGENT_MODEL=claude-opus-5-5"
            ]
        );
    }

    #[test]
    fn no_model_leaves_it_to_claude_code() {
        assert_eq!(args(""), Vec::<String>::new());
    }
}
