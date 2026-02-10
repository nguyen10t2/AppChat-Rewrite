use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

use crate::api::error;
use crate::modules::file_upload::{
    model::{NewFile, UploadConfig},
    repository::FileRepository,
    schema::{FileEntity, FileUploadResponse},
};

#[derive(Clone)]
pub struct FileUploadService<R>
where
    R: FileRepository + Send + Sync,
{
    file_repo: Arc<R>,
    config: UploadConfig,
}

impl<R> FileUploadService<R>
where
    R: FileRepository + Send + Sync,
{
    pub fn new(file_repo: Arc<R>, config: UploadConfig) -> Self {
        Self { file_repo, config }
    }

    pub fn with_defaults(file_repo: Arc<R>) -> Self {
        Self::new(file_repo, UploadConfig::default())
    }

    /// Validate file type and size
    fn validate_file(
        &self,
        _filename: &str,
        file_size: usize,
        mime_type: &str,
    ) -> Result<(), error::SystemError> {
        // Check file size
        if file_size > self.config.max_file_size {
            return Err(error::SystemError::bad_request(format!(
                "File size exceeds maximum allowed size of {} bytes",
                self.config.max_file_size
            )));
        }

        // Check MIME type
        if !self.config.allowed_mime_types.contains(&mime_type.to_string()) {
            return Err(error::SystemError::bad_request(format!(
                "File type '{}' is not allowed",
                mime_type
            )));
        }

        Ok(())
    }

    /// Generate unique filename
    fn generate_filename(&self, original_filename: &str) -> String {
        let extension =
            Path::new(original_filename).extension().and_then(|ext| ext.to_str()).unwrap_or("");
        let uuid = Uuid::now_v7();
        if extension.is_empty() {
            uuid.to_string()
        } else {
            format!("{}.{}", uuid, extension)
        }
    }

    /// Save file to disk
    async fn save_file(&self, filename: &str, bytes: &[u8]) -> Result<String, error::SystemError> {
        // Create upload directory if it doesn't exist
        tokio::fs::create_dir_all(&self.config.upload_dir).await?;

        let file_path = format!("{}/{}", self.config.upload_dir, filename);
        tokio::fs::write(&file_path, bytes).await?;

        Ok(file_path)
    }

    /// Upload file and save metadata
    pub async fn upload_file(
        &self,
        original_filename: String,
        bytes: Vec<u8>,
        mime_type: String,
        uploaded_by: Uuid,
    ) -> Result<FileUploadResponse, error::SystemError> {
        let file_size = bytes.len();

        // Validate file
        self.validate_file(&original_filename, file_size, &mime_type)?;

        // Generate unique filename
        let filename = self.generate_filename(&original_filename);

        // Save file to disk
        let storage_path = self.save_file(&filename, &bytes).await?;

        // Save metadata to database
        let mut tx = self.file_repo.get_pool().begin().await?;

        let new_file = NewFile {
            filename: filename.clone(),
            original_filename,
            mime_type,
            file_size: file_size as i64,
            storage_path,
            uploaded_by,
        };

        let file_entity = self.file_repo.create(&new_file, &mut *tx).await?;
        tx.commit().await?;

        // Build response
        let url = format!("{}/{}", self.config.base_url, filename);
        Ok(FileUploadResponse {
            id: file_entity.id,
            filename: file_entity.filename,
            original_filename: file_entity.original_filename,
            mime_type: file_entity.mime_type,
            file_size: file_entity.file_size,
            url,
            created_at: file_entity.created_at,
        })
    }

    /// Get file metadata by ID
    pub async fn get_file(&self, file_id: &Uuid) -> Result<Option<FileEntity>, error::SystemError> {
        self.file_repo.find_by_id(file_id).await
    }

    /// Delete file
    pub async fn delete_file(&self, file_id: &Uuid) -> Result<(), error::SystemError> {
        // Get file metadata first
        let file = self
            .file_repo
            .find_by_id(file_id)
            .await?
            .ok_or_else(|| error::SystemError::not_found("File not found"))?;

        // Delete from disk
        tokio::fs::remove_file(&file.storage_path).await.ok();

        // Delete from database
        let mut tx = self.file_repo.get_pool().begin().await?;
        self.file_repo.delete(file_id, &mut *tx).await?;
        tx.commit().await?;

        Ok(())
    }
}
