use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Something only the user can do: an errand, a booking, "make dinner on Friday".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Todo {
    pub id: i64,
    pub title: String,
    pub area: String,
    pub due: Option<NaiveDate>,
    /// A page it's about, e.g. a booking form.
    pub link: Option<String>,
    pub done_at: Option<DateTime<Utc>>,
}

/// A link as typed, with a blank one meaning none.
pub fn link_or_none(link: Option<String>) -> Option<String> {
    link.map(|l| l.trim().to_owned()).filter(|l| !l.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blank_link_is_none() {
        assert_eq!(link_or_none(Some("  ".to_owned())), None);
        assert_eq!(
            link_or_none(Some(" https://x.test ".to_owned())).as_deref(),
            Some("https://x.test")
        );
    }
}
