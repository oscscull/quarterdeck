//! Which done items have left the board for the archive.

use chrono::{DateTime, Duration, Utc};

use super::item::{Stage, WorkItem};

/// Done items finished more than `after` ago are archived; open work never is.
pub fn is_archived(item: &WorkItem, now: DateTime<Utc>, after: Duration) -> bool {
    item.stage() == Stage::Done && item.state_since < now - after
}

/// Keeps the board's items (open, or done within `after`) or the archive's, newest first.
pub fn select(
    mut items: Vec<WorkItem>,
    archived: bool,
    now: DateTime<Utc>,
    after: Duration,
) -> Vec<WorkItem> {
    items.retain(|i| is_archived(i, now, after) == archived);
    if archived {
        items.sort_by_key(|i| std::cmp::Reverse(i.state_since));
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::life::Life;

    fn done(id: &str, days_ago: i64, now: DateTime<Utc>) -> WorkItem {
        let mut item = WorkItem::sample(id, Life::Done);
        item.state_since = now - Duration::days(days_ago);
        item
    }

    #[test]
    fn done_items_archive_after_the_wait() {
        let now = Utc::now();
        let two = Duration::days(2);
        assert!(!is_archived(&done("fresh", 1, now), now, two));
        assert!(is_archived(&done("old", 3, now), now, two));
    }

    #[test]
    fn open_work_never_archives() {
        let now = Utc::now();
        let mut item = WorkItem::sample("open", Life::not_started());
        item.state_since = now - Duration::days(90);
        assert!(!is_archived(&item, now, Duration::days(2)));
    }

    #[test]
    fn board_and_archive_split_the_items() {
        let now = Utc::now();
        let open = WorkItem::sample("open", Life::not_started());
        let items = vec![
            open,
            done("fresh", 1, now),
            done("older", 5, now),
            done("newer", 3, now),
        ];
        let ids = |v: Vec<WorkItem>| v.into_iter().map(|i| i.id).collect::<Vec<_>>();
        let two = Duration::days(2);
        assert_eq!(
            ids(select(items.clone(), false, now, two)),
            ["open", "fresh"]
        );
        assert_eq!(ids(select(items, true, now, two)), ["newer", "older"]);
    }
}
