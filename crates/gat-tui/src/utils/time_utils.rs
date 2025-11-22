/// Time and timestamp utilities
///
/// Provides convenient wrappers around chrono for working with timestamps,
/// durations, and date/time formatting in the TUI.

use chrono::{DateTime, Utc, Local, Duration};
use serde::{Serialize, Deserialize};
use std::fmt;

/// Application timestamp type
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Create a new timestamp from current UTC time
    pub fn now() -> Self {
        Timestamp(Utc::now())
    }

    /// Create a timestamp from UTC datetime
    pub fn from_utc(dt: DateTime<Utc>) -> Self {
        Timestamp(dt)
    }

    /// Get the underlying DateTime<Utc>
    pub fn as_utc(&self) -> DateTime<Utc> {
        self.0
    }

    /// Format as ISO 8601 string
    pub fn to_iso8601(&self) -> String {
        self.0.to_rfc3339()
    }

    /// Format as human-readable local time
    pub fn to_local_string(&self) -> String {
        self.0.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string()
    }

    /// Format as short time (HH:MM:SS)
    pub fn to_time_string(&self) -> String {
        self.0.format("%H:%M:%S").to_string()
    }

    /// Get duration since this timestamp
    pub fn elapsed(&self) -> Duration {
        Timestamp::now().0 - self.0
    }

    /// Get milliseconds since this timestamp
    pub fn elapsed_ms(&self) -> i64 {
        self.elapsed().num_milliseconds()
    }

    /// Get seconds since this timestamp
    pub fn elapsed_secs(&self) -> i64 {
        self.elapsed().num_seconds()
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Timestamp::now()
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_local_string())
    }
}

/// Elapsed time display helper
#[derive(Clone, Copy, Debug)]
pub struct ElapsedTime {
    duration: Duration,
}

impl ElapsedTime {
    /// Create from a duration
    pub fn new(duration: Duration) -> Self {
        ElapsedTime { duration }
    }

    /// Create from milliseconds
    pub fn from_ms(ms: i64) -> Self {
        ElapsedTime {
            duration: Duration::milliseconds(ms),
        }
    }

    /// Create from seconds
    pub fn from_secs(secs: i64) -> Self {
        ElapsedTime {
            duration: Duration::seconds(secs),
        }
    }

    /// Get human-readable string representation
    pub fn to_string_human(&self) -> String {
        let secs = self.duration.num_seconds();
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        }
    }

    /// Get milliseconds
    pub fn as_millis(&self) -> i64 {
        self.duration.num_milliseconds()
    }

    /// Get seconds
    pub fn as_secs(&self) -> i64 {
        self.duration.num_seconds()
    }
}

impl fmt::Display for ElapsedTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_human())
    }
}

/// Time range for filtering or querying
#[derive(Clone, Debug)]
pub struct TimeRange {
    pub start: Timestamp,
    pub end: Timestamp,
}

impl TimeRange {
    /// Create a time range
    pub fn new(start: Timestamp, end: Timestamp) -> Self {
        TimeRange { start, end }
    }

    /// Create a range for the last N hours
    pub fn last_hours(hours: i64) -> Self {
        let now = Timestamp::now();
        let start = Timestamp(now.0 - Duration::hours(hours));
        TimeRange { start, end: now }
    }

    /// Create a range for the last N days
    pub fn last_days(days: i64) -> Self {
        let now = Timestamp::now();
        let start = Timestamp(now.0 - Duration::days(days));
        TimeRange { start, end: now }
    }

    /// Check if a timestamp is within this range
    pub fn contains(&self, ts: Timestamp) -> bool {
        ts.as_utc() >= self.start.as_utc() && ts.as_utc() <= self.end.as_utc()
    }

    /// Get the duration of this range
    pub fn duration(&self) -> Duration {
        self.end.as_utc() - self.start.as_utc()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_now() {
        let ts = Timestamp::now();
        assert!(ts.elapsed_secs() >= 0);
    }

    #[test]
    fn test_timestamp_iso8601() {
        let ts = Timestamp::now();
        let iso = ts.to_iso8601();
        assert!(iso.contains("T"));
        assert!(iso.contains("+") || iso.contains("Z")); // RFC3339 format may use +00:00 or Z
    }

    #[test]
    fn test_timestamp_local_string() {
        let ts = Timestamp::now();
        let local = ts.to_local_string();
        assert!(local.contains("-"));
        assert!(local.contains(":"));
    }

    #[test]
    fn test_timestamp_elapsed() {
        let ts = Timestamp::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = ts.elapsed_ms();
        assert!(elapsed > 0);
    }

    #[test]
    fn test_elapsed_time_human() {
        let et = ElapsedTime::from_secs(125);
        let s = et.to_string_human();
        assert!(s.contains("m"));
        assert!(s.contains("s"));
    }

    #[test]
    fn test_elapsed_time_short() {
        let et = ElapsedTime::from_secs(30);
        let s = et.to_string_human();
        assert_eq!(s, "30s");
    }

    #[test]
    fn test_elapsed_time_long() {
        let et = ElapsedTime::from_secs(7325); // 2h 2m 5s
        let s = et.to_string_human();
        assert!(s.contains("h"));
    }

    #[test]
    fn test_time_range_last_hours() {
        let before = Timestamp::now();
        let range = TimeRange::last_hours(1);
        // The range was created after before, so before should be after range's start
        // (accounting for the possibility that now is slightly different from when range was created)
        assert!(before.as_utc() >= range.start.as_utc());
    }

    #[test]
    fn test_time_range_contains() {
        let start = Timestamp::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mid = Timestamp::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let end = Timestamp::now();

        let range = TimeRange::new(start, end);
        assert!(range.contains(mid));
    }

    #[test]
    fn test_time_range_duration() {
        let range = TimeRange::last_hours(2);
        let duration_secs = range.duration().num_seconds();
        assert!(duration_secs > 7100 && duration_secs < 7300); // ~2 hours
    }

    #[test]
    fn test_timestamp_serialization() {
        use serde_json;
        let ts = Timestamp::now();
        let json = serde_json::to_string(&ts).expect("should serialize");
        let ts2: Timestamp = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(ts, ts2);
    }
}
