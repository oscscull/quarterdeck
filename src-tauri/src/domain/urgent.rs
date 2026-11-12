use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Something that went wrong or can't wait. Shown on Today until dismissed or resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Urgent {
    pub id: i64,
    pub title: String,
    pub detail: String,
    pub area: String,
    pub raised_at: DateTime<Utc>,
    /// Set when it is about a schedule, which lets the UI offer "Run now".
    pub schedule_id: Option<String>,
    /// Set when it is about a failed agent run, which lets the UI offer "Retry".
    pub run_id: Option<i64>,
}
