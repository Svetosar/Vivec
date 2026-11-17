use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Единая ошибка приложения. Всё, что может пойти не так в хендлере,
/// превращается в один из этих вариантов и отдаётся клиенту как HTTP-ответ.
#[derive(Debug)]
pub enum AppError {
    Template(minijinja::Error),
    Db(sqlx::Error),
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
        };
        (code, msg).into_response()
    }
}
