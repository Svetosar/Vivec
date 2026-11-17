use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Единая ошибка приложения. Всё, что может пойти не так в хендлере,
/// превращается в один из этих вариантов и отдаётся клиенту как HTTP-ответ.
#[derive(Debug)]
pub enum AppError {
    Template(minijinja::Error),
}

/// Позволяет писать `?` на результатах minijinja внутри хендлеров.
impl From<minijinja::Error> for AppError {
    fn from(e: minijinja::Error) -> Self {
        AppError::Template(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Template(e) => {
                // Подробности — в лог, клиенту — только код.
                tracing::error!("template error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, "500").into_response()
            }
        }
    }
}
