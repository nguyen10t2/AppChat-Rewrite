use uuid::Uuid;

/// New file metadata to insert into database
#[derive(Debug, Clone)]
pub struct NewFile {
    pub filename: String,
    pub original_filename: String,
    pub mime_type: String,
    pub file_size: i64,
    pub storage_path: String,
    pub uploaded_by: Uuid,
}

/// File upload configuration
#[derive(Debug, Clone)]
pub struct UploadConfig {
    pub max_file_size: usize,
    pub allowed_mime_types: Vec<String>,
    pub upload_dir: String,
    pub base_url: String,
}

impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            allowed_mime_types: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/gif".to_string(),
                "image/webp".to_string(),
                "application/pdf".to_string(),
                "text/plain".to_string(),
            ],
            upload_dir: "./uploads".to_string(),
            base_url: "/uploads".to_string(),
        }
    }
}
