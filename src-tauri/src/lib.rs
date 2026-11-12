//! Quarterdeck: tracks work in flight and runs recurring Claude jobs.

mod alerts;
mod answer;
mod app;
mod attention;
mod claude;
mod cli;
mod commands;
mod config;
mod connectors;
mod cron;
mod desktop;
mod devseed;
mod domain;
mod error;
mod jobs;
mod lasting;
mod lifecycle;
mod mcp;
mod notify;
mod park;
mod paths;
mod peers;
mod priorities;
mod profile;
mod queue;
mod rename;
mod retry;
mod runlog;
mod scheduler;
mod store;
mod today;
mod turns;
mod views;
mod work;
mod workspace;

use tauri::Manager as _;

/// Starts serving peers when `[sync]` is enabled, making a token the first time.
fn start_sync(config: &config::Config) -> error::AppResult<()> {
    let sync = config::settings::load(config)?.sync;
    if !sync.enabled {
        return Ok(());
    }
    start_token(config)?;
    peers::server::spawn(sync.listen())
}

/// Makes and saves the sync token the first time it's needed.
fn start_token(config: &config::Config) -> error::AppResult<()> {
    if config::settings::load(config)?.sync.token.is_empty() {
        config::edit::set_sync_token(config, &peers::new_token()?)?;
    }
    Ok(())
}

fn settings_for_login(config: &config::Config) -> bool {
    config::settings::load(config).map_or(true, |s| s.launch_at_login)
}

/// The old database stays where it is if importing it failed, and the user is told.
fn report_import(handle: &tauri::AppHandle, imported: error::AppResult<bool>) {
    if let Err(e) = imported {
        let flag = domain::urgent::Urgent {
            id: 0,
            title: "Couldn't bring over your old Quarterdeck data".to_owned(),
            detail: format!("{e}. The old database is untouched; tell Claude to look into it."),
            area: String::new(),
            raised_at: chrono::Utc::now(),
            schedule_id: None,
            run_id: None,
        };
        let db = &handle.state::<app::App>().db;
        // Telling the user is best effort; the app opens either way.
        drop(tauri::async_runtime::block_on(store::urgent::raise(
            db, &flag,
        )));
    }
}

/// Opens the database and config folder, then hands both to every command.
fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data = paths::data_dir()?;
    let db = tauri::async_runtime::block_on(store::Db::open(&data))?;
    let imported = tauri::async_runtime::block_on(store::legacy::import(&db, &data));
    // Only the app runs Claude, so anything still marked running is from before this start.
    tauri::async_runtime::block_on(store::runs::close_interrupted(&db, chrono::Utc::now()))?;
    // Leftover step logs only fill a folder; they never stop the app opening.
    drop(runlog::dir().and_then(|d| runlog::clear(&d)));
    let config = config::Config::open(paths::config_dir()?)?;
    let launch_at_login = settings_for_login(&config);
    // Serving peers is optional; a busy port must not stop the app opening.
    drop(start_sync(&config));
    app.manage(app::App { db, config });
    report_import(app.handle(), imported);
    desktop::tray::build(app.handle())?;
    // Launch-at-login is a convenience; failing to set it must not stop the app opening.
    drop(desktop::login::apply(app.handle(), launch_at_login));
    cli::warm(&["claude", "gh"]);
    scheduler::spawn(app.handle().clone());
    Ok(())
}

/// Gets the machine ready before any entry point runs: moves an install from the old name.
/// False if that failed, in which case nothing should open the config or database.
#[must_use]
pub fn prepare() -> bool {
    // The old install belongs to the installed app; a dev build must never move it into its own.
    profile::DEV || rename::migrate().is_ok()
}

/// Serves the Model Context Protocol over stdio instead of opening the app. Started as
/// `<binary> mcp`, so any Claude session can read and change Quarterdeck.
#[must_use]
pub fn serve_mcp() -> std::process::ExitCode {
    mcp::serve()
}

/// Runs one Quarterdeck tool from the command line: `<binary> tool <name> ['<json args>']`.
#[must_use]
pub fn run_tool() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(2).collect();
    mcp::command_line::main(
        args.first().map(String::as_str),
        args.get(1).map(String::as_str),
    )
}

/// Replaces this dev build's config and data with a safe copy of the installed app's. Started as
/// `<dev binary> seed-dev`, i.e. `npm run dev:seed`.
#[must_use]
pub fn seed_dev() -> std::process::ExitCode {
    use std::io::Write as _;
    let (code, message) = match tauri::async_runtime::block_on(devseed::seed()) {
        Ok(()) => (
            std::process::ExitCode::SUCCESS,
            "Copied the installed app's config and data into the dev build (paused, no peers, no Claude).".to_owned(),
        ),
        Err(e) => (std::process::ExitCode::FAILURE, format!("Couldn't seed the dev build: {e}")),
    };
    // Nowhere else to report to from the command line.
    drop(writeln!(std::io::stderr(), "{message}"));
    code
}

/// Serves this machine's Quarterdeck to peers in the foreground, without the app window. Started as
/// `<binary> serve`, e.g. on a machine without a screen.
#[must_use]
pub fn serve_peers() -> std::process::ExitCode {
    let started = paths::config_dir()
        .and_then(config::Config::open)
        .and_then(|config| {
            start_token(&config)?;
            peers::server::run(config::settings::load(&config)?.sync.listen())
        });
    if started.is_ok() {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}

/// Builds and runs the Tauri application.
///
/// # Errors
/// Returns an error if the Tauri runtime fails to start.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
#[allow(
    clippy::exit,
    reason = "generate_context! expands to code calling process::exit; not ours"
)]
pub fn run() -> tauri::Result<()> {
    let builder = tauri::Builder::default()
        // Must come first. Launching Quarterdeck while it runs shows the running one, so there's
        // never a second scheduler on the same database.
        .plugin(tauri_plugin_single_instance::init(|handle, _, _| {
            desktop::show_main(handle);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .on_window_event(desktop::on_window_event)
        .setup(setup)
        .invoke_handler(commands::handler());
    builder.run(context())
}

/// The app's config, renamed for a dev build so it's its own app beside the installed one.
#[allow(
    clippy::exit,
    reason = "tauri::generate_context! exits on a broken tauri.conf.json; that code is Tauri's"
)]
fn context() -> tauri::Context {
    let mut context = tauri::generate_context!();
    if profile::DEV {
        let config = context.config_mut();
        profile::APP_ID.clone_into(&mut config.identifier);
        config.product_name = Some(profile::NAME.to_owned());
        for window in &mut config.app.windows {
            profile::NAME.clone_into(&mut window.title);
        }
    }
    context
}
