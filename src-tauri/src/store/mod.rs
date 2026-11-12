//! Storage for live state: items, asks, meetings, activity and runs. The tables and every query
//! are defined in Based (`schema/`); these modules turn the generated client's rows into domain
//! types. Behaviour (schedules, prompts, policies) lives in files instead; see `config`.

mod client;
mod convert;
mod db;
mod snapshot;
mod time;

pub mod activity;
pub mod alert_state;
pub mod asks;
pub mod commitments;
pub mod events;
pub mod expire;
pub mod fired;
pub mod items;
pub mod legacy;
pub mod meta;
pub mod run_prompts;
pub mod run_requests;
pub mod runs;
pub mod sync_state;
pub mod todos;
pub mod urgent;
pub mod watch;

pub use db::Db;
pub use snapshot::snapshot;
