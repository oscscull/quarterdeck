//! Which Claude Code to run, the model every run is locked to, and the account it logs in as.

use tokio::process::Command;

/// Claude Code's override for the model its subagents use, which otherwise may differ per agent.
const SUBAGENT_MODEL: &str = "CLAUDE_CODE_SUBAGENT_MODEL";

/// How to start Claude Code for a headless run.
#[derive(Debug, Clone)]
pub struct Program {
    pub command: String,
    /// The model every run uses, subagents included. Empty leaves it to Claude Code.
    pub model: String,
    /// The account Claude Code finds its login under. Every run gets it as `USER` and `LOGNAME`,
    /// never the one Quarterdeck inherited.
    pub user: String,
}

impl Program {
    /// Runs `cmd` as `user`, whatever `USER` Quarterdeck inherited.
    pub fn as_user(&self, cmd: &mut Command) {
        cmd.env("USER", &self.user).env("LOGNAME", &self.user);
    }

    /// Pins `cmd` to the account and the model, so neither the inherited `USER` nor Claude Code's
    /// defaults nor a subagent can pick another.
    pub fn lock(&self, cmd: &mut Command) {
        self.as_user(cmd);
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
            user: "oscar".to_owned(),
        };
        let mut cmd = Command::new("claude");
        program.lock(&mut cmd);
        let std = cmd.as_std();
        let mut seen: Vec<String> = std
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        let mut envs: Vec<String> = std
            .get_envs()
            .filter_map(|(k, v)| Some(format!("{}={}", k.to_str()?, v?.to_str()?)))
            .collect();
        envs.sort();
        seen.extend(envs);
        seen
    }

    #[test]
    fn a_model_locks_the_run_and_its_subagents() {
        assert_eq!(
            args("claude-opus-5-5"),
            [
                "--model",
                "claude-opus-5-5",
                "CLAUDE_CODE_SUBAGENT_MODEL=claude-opus-5-5",
                "LOGNAME=oscar",
                "USER=oscar"
            ]
        );
    }

    #[test]
    fn no_model_leaves_it_to_claude_code_but_still_sets_the_account() {
        assert_eq!(args(""), ["LOGNAME=oscar", "USER=oscar"]);
    }
}
