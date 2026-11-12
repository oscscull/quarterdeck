//! Thin Tauri command handlers. Each one loads, delegates, and returns.
#![allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands must take their arguments and State by value"
)]
#![allow(
    clippy::let_underscore_must_use,
    reason = "tauri::command's async expansion discards a value; not our code"
)]

pub mod alerts;
pub mod areas;
pub mod briefing;
pub mod calendars;
pub mod chat;
pub mod commitments;
pub mod connect;
pub mod items;
pub mod outage;
pub mod peers;
pub mod runs;
pub mod schedules;
pub mod screens;
pub mod todos;
pub mod urgent;

/// Every command the frontend can call.
#[expect(
    clippy::too_many_lines,
    reason = "one flat list of commands; `generate_handler!` takes them all in one call"
)]
pub fn handler() -> impl Fn(tauri::ipc::Invoke) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        areas::areas,
        peers::machine,
        peers::peers,
        peers::peer_call,
        alerts::due_alert,
        alerts::alert_done,
        alerts::alert_snooze,
        connect::claude_connection,
        calendars::calendars,
        calendars::add_calendar,
        calendars::mac_calendars,
        calendars::add_mac_calendar,
        calendars::remove_calendar,
        screens::today,
        briefing::briefing,
        briefing::correct_briefing,
        commitments::add_commitment,
        commitments::delete_commitment,
        commitments::hide_commitment,
        todos::add_todo,
        todos::set_todo_done,
        todos::update_todo,
        todos::delete_todo,
        screens::work_items,
        screens::agent,
        runs::live_runs,
        runs::run_log,
        items::reorder_items,
        items::save_item,
        items::delete_item,
        items::respond,
        items::item_detail,
        items::park_item,
        items::unpark_item,
        items::parked_items,
        schedules::set_schedule_enabled,
        schedules::save_schedule,
        schedules::delete_schedule,
        schedules::explain_cron,
        schedules::event_kinds,
        schedules::approve_script,
        schedules::run_schedule_now,
        schedules::pause_all,
        schedules::resume_all,
        outage::check_agent,
        urgent::dismiss_urgent,
        urgent::dismiss_all_urgent,
        urgent::resolve_urgent,
        urgent::retry_urgent,
        chat::ask_claude,
        chat::draft_schedule,
    ]
}
