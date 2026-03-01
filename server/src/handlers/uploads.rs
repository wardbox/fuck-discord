use axum::{
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use serde::Serialize;
use std::path::PathBuf;

use crate::auth::middleware::AuthUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10MB
const ALLOWED_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "svg",
    "txt", "pdf", "zip", "gz", "tar",
    "mp3", "mp4", "webm", "ogg",
];

#[derive(Serialize)]
pub struct UploadResponse {
    pub url: String,
    pub filename: String,
    pub size: usize,
    pub content_type: String,
}

pub async fn upload_file(
    State(state): State<AppState>,
    Extension(_auth_user): Extension<AuthUser>,
    mut multipart: Multipart,
) -> AppResult<Json<UploadResponse>> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid multipart: {e}")))?
    {
        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "upload".to_string());

        // Derive MIME type from extension server-side (don't trust client)
        let _ = field.content_type(); // consume but ignore client-supplied type

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Failed to read file: {e}")))?;

        if data.len() > MAX_FILE_SIZE {
            return Err(AppError::BadRequest(format!(
                "File too large (max {}MB)",
                MAX_FILE_SIZE / 1024 / 1024
            )));
        }

        // Validate file extension (must be present and in allowlist)
        let ext = PathBuf::from(&filename)
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if ext.is_empty() || !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
            return Err(AppError::BadRequest(
                if ext.is_empty() {
                    "File must have a recognized extension".to_string()
                } else {
                    format!("File type .{ext} not allowed")
                }
            ));
        }

        // ext is guaranteed non-empty after validation above
        let unique_name = format!("{}.{ext}", ulid::Ulid::new());

        // Derive content type from validated extension
        let content_type = mime_guess::from_ext(&ext)
            .first_or_octet_stream()
            .to_string();

        // Save file
        let file_path = state.uploads_dir.join(&unique_name);
        tokio::fs::write(&file_path, &data)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to save file: {e}")))?;

        return Ok(Json(UploadResponse {
            url: format!("/uploads/{unique_name}"),
            filename,
            size: data.len(),
            content_type,
        }));
    }

    Err(AppError::BadRequest("No file provided".to_string()))
}

pub async fn serve_upload(
    State(state): State<AppState>,
    Path(filename): Path<String>,
) -> impl IntoResponse {
    // Sanitize filename — no path traversal
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return StatusCode::BAD_REQUEST.into_response();
    }

    let file_path = state.uploads_dir.join(&filename);
    match tokio::fs::read(&file_path).await {
        Ok(data) => {
            let content_type = mime_guess::from_path(&filename)
                .first_or_octet_stream()
                .to_string();

            // Force download for non-image types to prevent stored XSS
            let is_image = content_type.starts_with("image/") && content_type != "image/svg+xml";
            let disposition = if is_image {
                "inline".to_string()
            } else {
                format!("attachment; filename=\"{}\"", filename)
            };

            (
                [
                    (header::CONTENT_TYPE, content_type),
                    (
                        header::CACHE_CONTROL,
                        "public, max-age=31536000, immutable".to_string(),
                    ),
                    (
                        header::X_CONTENT_TYPE_OPTIONS,
                        "nosniff".to_string(),
                    ),
                    (
                        header::CONTENT_DISPOSITION,
                        disposition,
                    ),
                ],
                data,
            )
                .into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
