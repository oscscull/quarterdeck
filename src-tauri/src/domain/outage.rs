use chrono::{DateTime, Utc};
use serde::Serialize;

/// The agent can't run at all, so its work is waiting. Shown as one banner on Today until a check
/// finds it back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outage {
    pub why: String,
    pub since: DateTime<Utc>,
    /// When Quarterdeck checks again by itself.
    pub next_check: Option<DateTime<Utc>>,
}
