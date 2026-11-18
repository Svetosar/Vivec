use std::path::PathBuf;

use axum::{
    Json,
    extract::{Multipart, State},
};
use serde::Serialize;
use sqlx::sqlite::SqlitePool;

use crate::{App, db, error::AppError};

pub const MAX_UPLOAD: usize = 8 * 1024 * 1024;
const MAX_UPLOADS_TOTAL: u64 = 256 * 1024 * 1024;

#[derive(Serialize)]
pub struct UploadResponse {
    url: String,
}

pub async fn init() -> std::io::Result<PathBuf> {
    let dir = PathBuf::from(db::data_dir()).join("uploads");
    tokio::fs::create_dir_all(&dir).await?;
    Ok(dir)
}

pub async fn upload(
    State(_app): State<App>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("не удалось прочитать файл"))?
    {
        if field.name() != Some("file") {
            continue;
        }

        let bytes = field
            .bytes()
            .await
            .map_err(|_| AppError::BadRequest("не удалось прочитать файл"))?;
        if bytes.is_empty() || bytes.len() > MAX_UPLOAD {
            return Err(AppError::BadRequest("файл: от 1 байта до 8 МБ"));
        }

        let kind = infer::get(&bytes).ok_or(AppError::BadRequest("неизвестный тип файла"))?;
        let extension = match kind.mime_type() {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/webp" => "webp",
            "image/gif" => "gif",
            _ => return Err(AppError::BadRequest("разрешены JPEG, PNG, WebP и GIF")),
        };

        let dir = init()
            .await
            .map_err(|error| AppError::Internal(format!("uploads dir: {error}")))?;
        let used = uploads_size(&dir).await?;
        if used + bytes.len() as u64 > MAX_UPLOADS_TOTAL {
            return Err(AppError::BadRequest("общий лимит uploads исчерпан"));
        }

        let name = format!("{}.{}", random_name()?, extension);
        let path = dir.join(&name);
        tokio::fs::write(&path, bytes)
            .await
            .map_err(|error| AppError::Internal(format!("upload write: {error}")))?;

        return Ok(Json(UploadResponse {
            url: format!("/uploads/{name}"),
        }));
    }

    Err(AppError::BadRequest("поле file не найдено"))
}

async fn uploads_size(dir: &std::path::Path) -> Result<u64, AppError> {
    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|error| AppError::Internal(format!("uploads read: {error}")))?;
    let mut total = 0_u64;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|error| AppError::Internal(format!("uploads read: {error}")))?
    {
        let metadata = entry
            .metadata()
            .await
            .map_err(|error| AppError::Internal(format!("uploads stat: {error}")))?;
        if metadata.is_file() {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}

fn random_name() -> Result<String, AppError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|_| AppError::Internal("не удалось создать имя файла".to_string()))?;

    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut name = String::with_capacity(32);
    for byte in bytes {
        name.push(HEX[(byte >> 4) as usize] as char);
        name.push(HEX[(byte & 0x0f) as usize] as char);
    }
    Ok(name)
}

pub async fn cleanup_unused(pool: &SqlitePool, old_body: &str) {
    for name in upload_names(old_body) {
        let pattern = format!("%/uploads/{name}%");
        let used =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM articles WHERE body_md LIKE ?")
                .bind(pattern)
                .fetch_one(pool)
                .await;
        match used {
            Ok(0) => {
                let path = PathBuf::from(db::data_dir()).join("uploads").join(&name);
                if let Err(error) = tokio::fs::remove_file(&path).await
                    && error.kind() != std::io::ErrorKind::NotFound
                {
                    tracing::warn!("upload cleanup {}: {error}", path.display());
                }
            }
            Ok(_) => {}
            Err(error) => tracing::warn!("upload reference check {name}: {error}"),
        }
    }
}

fn upload_names(body: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("/uploads/") {
        rest = &rest[start + "/uploads/".len()..];
        let end = rest
            .find([')', '"', '\'', ' ', '\n', '\r'])
            .unwrap_or(rest.len());
        let name = &rest[..end];
        if !name.is_empty()
            && !name.contains('/')
            && name
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ".-_".contains(ch))
            && !names.iter().any(|known| known == name)
        {
            names.push(name.to_string());
        }
        rest = &rest[end..];
    }
    names
}

#[cfg(test)]
mod tests {
    use super::upload_names;

    #[test]
    fn finds_unique_upload_names() {
        let body = "![a](/uploads/one.png) ![b](/uploads/one.png) ![c](/uploads/two.webp)";
        assert_eq!(upload_names(body), vec!["one.png", "two.webp"]);
    }
}
