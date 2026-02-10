use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

/// File metadata entity from database
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FileEntity {
    pub id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub mime_type: String,
    pub file_size: i64,
    pub storage_path: String,
    pub uploaded_by: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// File upload request/response DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct FileUploadResponse {
    pub id: Uuid,
    pub filename: String,
    pub original_filename: String,
    pub mime_type: String,
    pub file_size: i64,
    pub url: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
