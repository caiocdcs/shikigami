use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Maximum number of message characters folded into the notification body.
///
/// The check-in row stores the full message (up to the ingress body limit), but
/// push channels (ntfy/gotify/slack) truncate their own displays, so a bounded
/// excerpt keeps the alert readable and the dashboard authoritative.
const NOTIFICATION_MESSAGE_EXCERPT: usize = 256;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationContent {
    pub title: String,
    pub body: String,
    pub monitor_name: String,
    pub monitor_slug: String,
    pub last_seen_at: Option<DateTime<Utc>>,
}

impl NotificationContent {
    pub fn for_failure(
        monitor_name: &str,
        monitor_slug: &str,
        last_seen_at: Option<DateTime<Utc>>,
        message: Option<&str>,
    ) -> Self {
        let last_seen = last_seen_at.map_or_else(|| "never".to_string(), |t| t.to_rfc3339());

        let body = match message.map(str::trim).filter(|m| !m.is_empty()) {
            Some(m) => {
                let excerpt = truncate_at_char_boundary(m, NOTIFICATION_MESSAGE_EXCERPT);
                format!(
                    "Monitor {monitor_name} ({monitor_slug}) reported failure. Last seen: {last_seen}. Reason: {excerpt}"
                )
            }
            None => format!(
                "Monitor {monitor_name} ({monitor_slug}) reported failure. Last seen: {last_seen}"
            ),
        };

        Self {
            title: monitor_name.to_string(),
            body,
            monitor_name: monitor_name.to_string(),
            monitor_slug: monitor_slug.to_string(),
            last_seen_at,
        }
    }
}

/// Truncate `s` to at most `max_chars` characters without splitting a Unicode
/// scalar value. `String` is UTF-8 and `char` iteration yields scalar values, so
/// this never lands inside a multi-byte sequence.
fn truncate_at_char_boundary(s: &str, max_chars: usize) -> &str {
    if s.chars().count() <= max_chars {
        return s;
    }
    s.char_indices()
        .nth(max_chars)
        .map_or(s, |(idx, _)| &s[..idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_failure_without_message_omits_reason() {
        let content = NotificationContent::for_failure("backup-job", "backup-job", None, None);
        assert_eq!(content.title, "backup-job");
        assert_eq!(content.monitor_name, "backup-job");
        assert_eq!(content.monitor_slug, "backup-job");
        assert!(content.body.contains("backup-job"));
        assert!(content.body.contains("never"));
        assert!(!content.body.contains("Reason"));
    }

    #[test]
    fn for_failure_with_message_appends_reason() {
        let content =
            NotificationContent::for_failure("backup-job", "backup-job", None, Some("exit 1"));
        assert!(content.body.contains("Reason: exit 1"));
    }

    #[test]
    fn for_failure_with_blank_message_omits_reason() {
        let content =
            NotificationContent::for_failure("backup-job", "backup-job", None, Some("   "));
        assert!(!content.body.contains("Reason"));
    }

    #[test]
    fn for_failure_with_last_seen() {
        let dt = chrono::DateTime::parse_from_rfc3339("2026-01-15T10:30:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let content = NotificationContent::for_failure("db-sync", "db-sync", Some(dt), None);
        assert!(content.body.contains("2026-01-15T10:30:00+00:00"));
    }

    #[test]
    fn for_failure_truncates_long_message_to_excerpt() {
        let long = "x".repeat(1_000);
        let content =
            NotificationContent::for_failure("backup-job", "backup-job", None, Some(&long));
        let reason_start = content.body.find("Reason: ").unwrap() + "Reason: ".len();
        let reason = &content.body[reason_start..];
        assert_eq!(reason.chars().count(), NOTIFICATION_MESSAGE_EXCERPT);
        assert!(reason.chars().all(|c| c == 'x'));
    }

    #[test]
    fn for_failure_truncates_on_unicode_boundary() {
        // Each emoji is one char (one scalar value). Truncation must not split one.
        let m = "ab".to_string() + &"\u{1F600}".repeat(1_000);
        let content = NotificationContent::for_failure("m", "m", None, Some(&m));
        let reason_start = content.body.find("Reason: ").unwrap() + "Reason: ".len();
        let reason = &content.body[reason_start..];
        // 2 ASCII + 254 emojis = 256 chars, no panic, valid UTF-8.
        assert_eq!(reason.chars().count(), NOTIFICATION_MESSAGE_EXCERPT);
        assert!(reason.is_char_boundary(reason.len()));
    }

    #[test]
    fn serializes_and_deserializes_roundtrip() {
        let dt = chrono::DateTime::parse_from_rfc3339("2026-01-15T10:30:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let original =
            NotificationContent::for_failure("backup", "backup", Some(dt), Some("exit 1"));
        let json = serde_json::to_string(&original).unwrap();
        let restored: NotificationContent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.title, original.title);
        assert_eq!(restored.body, original.body);
        assert_eq!(restored.monitor_name, original.monitor_name);
        assert_eq!(restored.monitor_slug, original.monitor_slug);
    }
}
