use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use sqlx::sqlite::SqlitePool;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod articles;
mod db;
mod error;
mod markdown;
mod slug;
use error::AppError;

const LAYOUT: &str = include_str!("../templates/layout.html.jinja");
const INDEX: &str = include_str!("../templates/index.html.jinja");
const ARTICLE: &str = include_str!("../templates/article.html.jinja");
const RUBRIC: &str = include_str!("../templates/rubric.html.jinja");
const NOT_FOUND: &str = include_str!("../templates/404.html.jinja");

/// Состояние приложения: одно на весь процесс, клонируется в каждый запрос.
#[derive(Clone)]
struct App {
    tpl: minijinja::Environment<'static>,
    pool: SqlitePool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = db::open().await?;

    let mut tpl = minijinja::Environment::new();
    tpl.add_template("layout.html.jinja", LAYOUT)?;
    tpl.add_template("index.html.jinja", INDEX)?;
    tpl.add_template("article.html.jinja", ARTICLE)?;
    tpl.add_template("rubric.html.jinja", RUBRIC)?;
    tpl.add_template("404.html.jinja", NOT_FOUND)?;

    let router = Router::new()
        .route("/", get(index))
        .route("/healthz", get(healthz))
        .route("/rubric/{rubric}", get(rubric))
        // Динамический маршрут последним: статические пути матчатся раньше.
        .route("/{slug}", get(article))
        .nest_service("/static", ServeDir::new("static"))
        .fallback(not_found)
        .layer(TraceLayer::new_for_http())
        .with_state(App { tpl, pool });

    // Порт берём из окружения: на этой машине 8080 занят steamwebhelper,
    // а за Caddy порт вообще может быть другим.
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{addr}");
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Корректная остановка: podman/systemd шлют SIGTERM и ждут 10 секунд,
/// после чего убивают процесс SIGKILL. Без этого обработчика каждый
/// рестарт — это 10 секунд простоя и обрыв запросов.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to listen for ctrl-c");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to listen for SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received, finishing in-flight requests");
}

/// Проверка живости для мониторинга и healthcheck в контейнере.
async fn healthz() -> &'static str {
    "ok"
}

/// Контекст главной. Отдельные структуры вместо `context!`: шаблон получает
/// ровно те поля, которые ему нужны, и видно имена.
#[derive(serde::Serialize)]
struct IndexCtx {
    hero: Option<articles::Article>,
    cards: Vec<articles::Article>,
}

/// Главная: первый материал крупно, следующие три — карточками.
async fn index(State(app): State<App>) -> Result<Html<String>, AppError> {
    let mut list = articles::published(&app.pool).await?;
    let mut cards = list.split_off(list.len().min(1));
    let hero = if list.is_empty() { None } else { Some(list.remove(0)) };
    cards.truncate(3);

    let ctx = minijinja::Value::from(minijinja::value::Serde(IndexCtx { hero, cards }));
    let html = app.tpl.get_template("index.html.jinja")?.render(ctx)?;
    Ok(Html(html))
}

/// Контекст страницы статьи.
#[derive(serde::Serialize)]
struct ArticleCtx<'a> {
    article: &'a articles::Article,
}

/// Страница статьи по slug. Черновик и несуществующий slug — одна и та же 404.
async fn article(State(app): State<App>, Path(slug): Path<String>) -> Result<Response, AppError> {
    match articles::by_slug(&app.pool, &slug).await? {
        Some(article) => {
            let ctx = minijinja::Value::from(minijinja::value::Serde(ArticleCtx { article: &article }));
            let html = app.tpl.get_template("article.html.jinja")?.render(ctx)?;
            Ok(Html(html).into_response())
        }
        None => Ok(not_found(State(app)).await),
    }
}

/// Контекст страницы рубрики.
#[derive(serde::Serialize)]
struct RubricCtx {
    rubric: &'static str,
    slug: String,
    articles: Vec<articles::Article>,
}

/// Страница рубрики: `/rubric/shkola`. Неизвестный slug — 404, а не пустой
/// список: URL должен существовать только для настоящих рубрик.
async fn rubric(
    State(app): State<App>,
    Path(rubric_slug): Path<String>,
) -> Result<Response, AppError> {
    let Some(name) = articles::rubric_name(&rubric_slug) else {
        return Ok(not_found(State(app)).await);
    };

    let list = articles::by_rubric(&app.pool, &rubric_slug).await?;
    let ctx = minijinja::Value::from(minijinja::value::Serde(RubricCtx {
        rubric: name,
        slug: rubric_slug,
        articles: list,
    }));
    let html = app.tpl.get_template("rubric.html.jinja")?.render(ctx)?;
    Ok(Html(html).into_response())
}

/// Страница 404. Отдельный хендлер, а не просто код: пользователь должен
/// видеть газету, а не белую строку «404».
async fn not_found(State(app): State<App>) -> Response {
    match app.tpl.get_template("404.html.jinja").and_then(|t| t.render(())) {
        Ok(html) => (StatusCode::NOT_FOUND, Html(html)).into_response(),
        Err(e) => {
            tracing::error!("404 template error: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "500").into_response()
        }
    }
}
