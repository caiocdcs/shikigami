use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CheckInResponse {
    pub id: String,
    pub monitor_id: String,
    pub checked_in_at: String,
    pub outcome: String,
    pub comments: Option<String>,
}
