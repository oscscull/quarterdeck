//! The account Claude Code runs as when `claude_user` is empty.

use crate::error::{AppError, AppResult};

/// This Mac's account name, from the OS account database rather than the environment.
pub fn this_macs() -> AppResult<String> {
    match nix::unistd::User::from_uid(nix::unistd::getuid()) {
        Ok(Some(user)) => Ok(user.name),
        Ok(None) => Err(AppError::Claude(
            "this Mac's account isn't in its account database; set claude_user in quarterdeck.toml"
                .to_owned(),
        )),
        Err(e) => Err(AppError::Claude(format!(
            "couldn't look up this Mac's account name ({e}); set claude_user in quarterdeck.toml"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_is_the_account_id_names() {
        let id = std::process::Command::new("id").arg("-un").output();
        let named = id.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
        assert_eq!(this_macs().ok(), named.ok());
    }
}
