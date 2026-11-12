// Prevents an extra console window on Windows in release. Do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    // Every entry point shares the config and database, so move an old install first.
    if !quarterdeck_lib::prepare() {
        return ExitCode::FAILURE;
    }
    match std::env::args().nth(1).as_deref() {
        Some("mcp") => return quarterdeck_lib::serve_mcp(),
        Some("serve") => return quarterdeck_lib::serve_peers(),
        Some("seed-dev") => return quarterdeck_lib::seed_dev(),
        Some("tool") => return quarterdeck_lib::run_tool(),
        _ => {}
    }
    match quarterdeck_lib::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
