use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    ) -> Self {
        let last_seen = last_seen_at.map_or_else(|| "never".to_string(), |t| t.to_rfc3339());

        Self {
            title: monitor_name.to_string(),
            body: format!(
                "Monitor {monitor_name} ({monitor_slug}) reported failure. Last seen: {last_seen}"
            ),
            monitor_name: monitor_name.to_string(),
            monitor_slug: monitor_slug.to_string(),
            last_seen_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_failure_composes_title_and_body() {
        let content = NotificationContent::for_failure("backup-job", "backup-job", None);
        assert_eq!(content.title, "backup-job");
        assert_eq!(content.monitor_name, "backup-job");
        assert_eq!(content.monitor_slug, "backup-job");
        assert!(content.body.contains("backup-job"));
        assert!(content.body.contains("never"));
    }

    #[test]
    fn for_failure_with_last_seen() {
        let dt = chrono::DateTime::parse_from_rfc3339("2026-01-15T10:30:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let content = NotificationContent::for_failure("db-sync", "db-sync", Some(dt));
        assert!(content.body.contains("2026-01-15T10:30:00+00:00"));
    }

    #[test]
    fn serializes_and_deserializes_roundtrip() {
        let dt = chrono::DateTime::parse_from_rfc3339("2026-01-15T10:30:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let original = NotificationContent::for_failure("backup", "backup", Some(dt));
        let json = serde_json::to_string(&original).unwrap();
        let restored: NotificationContent = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.title, original.title);
        assert_eq!(restored.body, original.body);
        assert_eq!(restored.monitor_name, original.monitor_name);
        assert_eq!(restored.monitor_slug, original.monitor_slug);
    }
}
