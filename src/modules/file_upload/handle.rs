use actix_multipart::Multipart;
use actix_web::web;
use futures_util::TryStreamExt;
use uuid::Uuid;

use crate::api::success::Success;
use crate::api::{error, success};
use crate::modules::file_upload::schema::FileUploadResponse;
use crate::modules::file_upload::service::FileUploadService;

/// Upload file handler
pub async fn upload_file<R>(
    mut payload: Multipart,
    req: actix_web::HttpRequest,
    service: web::Data<FileUploadService<R>>,
) -> Result<success::Success<FileUploadResponse>, error::Error>
where
    R: crate::modules::file_upload::repository::FileRepository + Send + Sync + 'static,
{
    let user_id = crate::middlewares::get_extensions::<crate::utils::Claims>(&req)?.sub;

    // Process multipart form data
    if let Some(mut field) = payload.try_next().await.map_err(|_| error::Error::InternalServer)? {
        let content_disposition = field
            .content_disposition()
            .ok_or_else(|| error::Error::bad_request("Missing content disposition"))?;

        let filename = content_disposition
            .get_filename()
            .ok_or_else(|| error::Error::bad_request("Missing filename"))?
            .to_string();

        // Detect MIME type
        let mime_type = field
            .content_type()
            .map(|m| m.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        // Read file bytes
        let mut bytes = Vec::new();
        while let Some(chunk) = field.try_next().await.map_err(|_| error::Error::InternalServer)? {
            bytes.extend_from_slice(&chunk);
        }

        // Upload file
        let result = service.upload_file(filename, bytes, mime_type, user_id).await?;

        return Ok(Success::ok(Some(result)).message("File uploaded successfully"));
    }

    Err(error::Error::bad_request("No file found in request"))
}

/// Get file metadata handler
pub async fn get_file<R>(
    file_id: web::Path<Uuid>,
    service: web::Data<FileUploadService<R>>,
) -> Result<success::Success<crate::modules::file_upload::schema::FileEntity>, error::Error>
where
    R: crate::modules::file_upload::repository::FileRepository + Send + Sync + 'static,
{
    let file_id = file_id.into_inner();

    match service.get_file(&file_id).await {
        Ok(Some(file)) => Ok(Success::ok(Some(file))),
        Ok(None) => Err(error::Error::not_found("File not found")),
        Err(e) => Err(error::Error::from(e)),
    }
}

/// Delete file handler
pub async fn delete_file<R>(
    file_id: web::Path<Uuid>,
    req: actix_web::HttpRequest,
    service: web::Data<FileUploadService<R>>,
) -> Result<success::Success<String>, error::Error>
where
    R: crate::modules::file_upload::repository::FileRepository + Send + Sync + 'static,
{
    let file_id = file_id.into_inner();
    let user_id = crate::middlewares::get_extensions::<crate::utils::Claims>(&req)?.sub;

    // Get file to check ownership
    match service.get_file(&file_id).await {
        Ok(Some(file)) => {
            // Check if user owns the file
            if file.uploaded_by != user_id {
                return Err(error::Error::forbidden(
                    "You don't have permission to delete this file",
                ));
            }

            // Delete file
            service.delete_file(&file_id).await?;
            Ok(Success::ok(Some("File deleted successfully".to_string()))
                .message("File deleted successfully"))
        }
        Ok(None) => Err(error::Error::not_found("File not found")),
        Err(e) => Err(error::Error::from(e)),
    }
}
