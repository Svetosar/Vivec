use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

/// Единая ошибка приложения. Всё, что может пойти не так в хендлере,
/// превращается в один из этих вариантов и отдаётся клиенту как HTTP-ответ.
#[derive(Debug)]
pub enum AppError {
    Template(minijinja::Error),
    Db(sqlx::Error),
    BadRequest(&'static str),
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict(&'static str),
    TooManyRequests,
    Unavailable(&'static str),
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
}

/// Позволяет писать `?` на результатах minijinja внутри хендлеров.
impl From<minijinja::Error> for AppError {
    fn from(e: minijinja::Error) -> Self {
        AppError::Template(e)
    }
}

/// Позволяет писать `?` на запросах sqlx.
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Db(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Подробности — в лог, клиенту — только код: в тексте ошибки базы
        // бывает и путь, и SQL.
        let (code, msg) = match self {
            AppError::Template(e) => {
                tracing::error!("template error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "500")
            }
            AppError::Db(e) => {
                tracing::error!("db error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "500")
            }
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "требуется вход"),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "запрос отклонён"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "не найдено"),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            AppError::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, "слишком много попыток"),
            AppError::Unavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            AppError::Internal(error) => {
                tracing::error!("internal error: {error}");
                (StatusCode::INTERNAL_SERVER_ERROR, "500")
            }
        };
        if code.is_client_error() || code == StatusCode::SERVICE_UNAVAILABLE {
            (code, Json(ErrorBody { error: msg })).into_response()
        } else {
            (code, msg).into_response()
        }
    }
}
