use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthUser {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUrl {
    pub original_url: String,
    pub custom_alias: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UrlSummary {
    pub id: Uuid,
    pub original_url: String,
    pub short_code: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub clicks: i64,
}

#[derive(Debug, Serialize)]
pub struct CreatedUrl {
    pub id: Uuid,
    pub original_url: String,
    pub short_code: String,
    pub short_url: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DailyClicks {
    pub day: chrono::NaiveDate,
    pub clicks: i64,
}

#[derive(Debug, Serialize)]
pub struct UrlAnalytics {
    pub short_code: String,
    pub total_clicks: i64,
    pub daily_clicks: Vec<DailyClicks>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedUrl {
    pub id: Uuid,
    pub original_url: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}
