use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, FromRow)]
pub struct Audio {
    pub id: Uuid,
    pub owner_id: String,
    pub title: String,
    pub description: Option<String>,
    pub filename: String,
    pub object_key: String,
    pub size_bytes: i64,
    pub duration_ms: Option<i32>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}


#[derive(Debug, Serialize, FromRow)]
pub struct AudioWithMeta {
    pub id: Uuid,
    pub owner_id: String,
    pub title: String,
    pub description: Option<String>,
    pub filename: String,
    pub size_bytes: i64,
    pub duration_ms: Option<i32>,
    pub is_public: bool,
    pub created_at: DateTime<Utc>,
    pub is_owned: bool,
}

#[derive(Debug, Deserialize)]
pub struct UploadAudioQuery {
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchAudioQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 { 20 }

#[derive(Debug, Deserialize)]
pub struct UpdatePrivacyRequest {
    pub is_public: bool,
}

#[derive(Debug, Serialize)]
pub struct AudioListResponse {
    pub items: Vec<AudioWithMeta>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub id: Uuid,
    pub title: String,
    pub object_key: String,
    pub size_bytes: i64,
}