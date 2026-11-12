//! `<app binary> tool <name> ['<json args>']`: runs one Quarterdeck tool and prints its result as
//! JSON, so scripts (and script schedules, through `$QUARTERDECK`) get every tool agents have.

use std::io::Write as _;
use std::process::ExitCode;

use serde_json::Value;

use super::{Ctx, call, registry};
use crate::error::{AppError, AppResult};

async fn run(name: &str, args: Option<&str>) -> AppResult<Value> {
    let args = match args {
        Some(text) => serde_json::from_str(text)
            .map_err(|e| AppError::BadInput(format!("the arguments aren't JSON: {e}")))?,
        None => Value::Object(serde_json::Map::new()),
    };
    let ctx = Ctx::open().await?;
    call(&ctx, &registry(), name, args).await
}

/// Runs the tool named on the command line. With no name, lists the tools.
pub fn main(name: Option<&str>, args: Option<&str>) -> ExitCode {
    let Some(name) = name else {
        let names: Vec<_> = registry().into_iter().map(|t| t.name).collect();
        let out = format!(
            "usage: quarterdeck tool <name> ['<json args>']\ntools: {}",
            names.join(", ")
        );
        // Nowhere else to report to from the command line.
        drop(writeln!(std::io::stderr(), "{out}"));
        return ExitCode::FAILURE;
    };
    match tauri::async_runtime::block_on(run(name, args)) {
        Ok(value) => {
            drop(writeln!(std::io::stdout(), "{value}"));
            ExitCode::SUCCESS
        }
        Err(e) => {
            drop(writeln!(std::io::stderr(), "{e}"));
            ExitCode::FAILURE
        }
    }
}
