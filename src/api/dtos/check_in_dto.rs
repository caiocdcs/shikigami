use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CheckInResponse {
    pub id: String,
    pub monitor_id: String,
    pub checked_in_at: String,
    pub outcome: String,
    pub comments: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckInsPage {
    pub items: Vec<CheckInResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}
