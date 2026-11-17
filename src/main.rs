use axum::{extract::State, response::Html, routing::get, Router};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod error;
use error::AppError;

const LAYOUT: &str = include_str!("../templates/layout.html.jinja");
const INDEX: &str = include_str!("../templates/index.html.jinja");

/// Состояние приложения: одно на весь процесс, клонируется в каждый запрос.
/// Пока здесь только шаблонизатор; на 16.11 добавится пул БД.
#[derive(Clone)]
struct App {
    tpl: minijinja::Environment<'static>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut tpl = minijinja::Environment::new();
    tpl.add_template("layout.html.jinja", LAYOUT)?;
    tpl.add_template("index.html.jinja", INDEX)?;

    let router = Router::new()
        .route("/", get(index))
        .route("/healthz", get(healthz))
        .nest_service("/static", ServeDir::new("static"))
        .layer(TraceLayer::new_for_http())
        .with_state(App { tpl });

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

async fn index(State(app): State<App>) -> Result<Html<String>, AppError> {
    let html = app.tpl.get_template("index.html.jinja")?.render(())?;
    Ok(Html(html))
}
