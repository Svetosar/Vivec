use std::path::PathBuf;

use axum::{
    Json,
    extract::{Multipart, State},
};
use serde::Serialize;

use crate::{App, db, error::AppError};

pub const MAX_UPLOAD: usize = 8 * 1024 * 1024;

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

        let name = format!("{}.{}", random_name()?, extension);
        let path = init()
            .await
            .map_err(|error| AppError::Internal(format!("uploads dir: {error}")))?
            .join(&name);
        tokio::fs::write(&path, bytes)
            .await
            .map_err(|error| AppError::Internal(format!("upload write: {error}")))?;

        return Ok(Json(UploadResponse {
            url: format!("/uploads/{name}"),
        }));
    }

    Err(AppError::BadRequest("поле file не найдено"))
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
