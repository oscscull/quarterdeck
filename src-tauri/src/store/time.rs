//! Times as stored: Based's canonical UTC form (six fractional digits, `Z`), so the database compares
//! and sorts them correctly. Based normalises `timestamp` columns to it; `meta` keeps times as text in
//! the same form.

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};

use super::client::{Date, Timestamp};
use crate::error::{AppError, AppResult};

const TIME: &str = "%Y-%m-%dT%H:%M:%S%.6fZ";
const DATE: &str = "%Y-%m-%d";

pub fn put(at: DateTime<Utc>) -> Timestamp {
    at.format(TIME).to_string()
}

pub fn put_opt(at: Option<DateTime<Utc>>) -> Option<Timestamp> {
    at.map(put)
}

pub fn get(stored: &str) -> AppResult<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(stored, TIME)
        .map(|t| t.and_utc())
        .map_err(|e| AppError::Db(format!("bad time {stored:?}: {e}")))
}

pub fn get_opt(stored: Option<&str>) -> AppResult<Option<DateTime<Utc>>> {
    stored.map(get).transpose()
}

pub fn put_date(day: NaiveDate) -> Date {
    day.format(DATE).to_string()
}

pub fn get_date(stored: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(stored, DATE)
        .map_err(|e| AppError::Db(format!("bad date {stored:?}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_times_round_trip_and_sort_in_time_order() {
        let early = get("2026-10-21T08:00:00.000000Z").unwrap();
        let late = early + chrono::Duration::milliseconds(1500);
        assert_eq!(get(&put(late)).unwrap(), late);
        assert!(put(early) < put(late));
        let day = get_date("2026-10-21").unwrap();
        assert_eq!(get_date(&put_date(day)).unwrap(), day);
    }
}
