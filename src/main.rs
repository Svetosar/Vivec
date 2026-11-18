use std::net::SocketAddr;

use axum::{
    Router,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use sqlx::sqlite::SqlitePool;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod articles;
mod auth;
mod db;
mod error;
mod markdown;
mod slug;
mod uploads;
use error::AppError;

const LAYOUT: &str = include_str!("../templates/layout.html.jinja");
const INDEX: &str = include_str!("../templates/index.html.jinja");
const ARTICLE: &str = include_str!("../templates/article.html.jinja");
const RUBRIC: &str = include_str!("../templates/rubric.html.jinja");
const SEARCH: &str = include_str!("../templates/search.html.jinja");
const ARCHIVE: &str = include_str!("../templates/archive.html.jinja");
const NOT_FOUND: &str = include_str!("../templates/404.html.jinja");
const FAVICON: &[u8] = include_bytes!("../static/favicon.ico");

/// Состояние приложения: одно на весь процесс, клонируется в каждый запрос.
#[derive(Clone)]
struct App {
    tpl: minijinja::Environment<'static>,
    pool: SqlitePool,
    auth: auth::Auth,
    public_origin: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = db::open().await?;
    let uploads_dir = uploads::init().await?;
    let public_origin = std::env::var("PUBLIC_ORIGIN")
        .unwrap_or_else(|_| "http://127.0.0.1:8081".to_string())
        .trim_end_matches('/')
        .to_string();

    let mut tpl = minijinja::Environment::new();
    tpl.add_template("layout.html.jinja", LAYOUT)?;
    tpl.add_template("index.html.jinja", INDEX)?;
    tpl.add_template("article.html.jinja", ARTICLE)?;
    tpl.add_template("rubric.html.jinja", RUBRIC)?;
    tpl.add_template("search.html.jinja", SEARCH)?;
    tpl.add_template("archive.html.jinja", ARCHIVE)?;
    tpl.add_template("404.html.jinja", NOT_FOUND)?;

    let app = App {
        tpl,
        pool,
        auth: auth::Auth::from_env(),
        public_origin,
    };

    let protected_admin = Router::new()
        .route("/me", get(auth::me))
        .route("/logout", post(auth::logout))
        .route("/articles", get(api::list).post(api::create))
        .route(
            "/articles/{id}",
            get(api::get).put(api::update).delete(api::delete),
        )
        .route("/articles/{id}/publish", post(api::publish))
        .route("/articles/{id}/unpublish", post(api::unpublish))
        .route(
            "/uploads",
            post(uploads::upload).layer(DefaultBodyLimit::max(uploads::MAX_UPLOAD + 64 * 1024)),
        )
        .route_layer(middleware::from_fn_with_state(
            app.clone(),
            auth::require_session,
        ));
    let admin_api = Router::new()
        .route("/login", post(auth::login))
        .merge(protected_admin)
        .route_layer(middleware::from_fn(auth::require_csrf))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ));

    let admin_files =
        ServeDir::new("static/admin").fallback(ServeFile::new("static/admin/index.html"));

    let router = Router::new()
        .route("/", get(index))
        .route("/healthz", get(healthz))
        .route("/rubric/{rubric}", get(rubric))
        .route("/search", get(search))
        .route("/archive", get(archive))
        .route("/robots.txt", get(robots_txt))
        .route("/sitemap.xml", get(sitemap_xml))
        .route("/favicon.ico", get(favicon))
        // Динамический маршрут последним: статические пути матчатся раньше.
        .route("/{slug}", get(article))
        .nest("/api/admin", admin_api)
        .nest_service("/admin", admin_files)
        .nest_service("/static", ServeDir::new("static"))
        .nest_service("/uploads", ServeDir::new(uploads_dir))
        .fallback(not_found)
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; \
                 script-src 'self'; object-src 'none'; frame-ancestors 'none'; \
                 base-uri 'self'; form-action 'self'",
            ),
        ))
        .layer(middleware::from_fn(cache_headers))
        .layer(TraceLayer::new_for_http())
        .with_state(app);

    // Порт берём из окружения: на этой машине 8080 занят steamwebhelper,
    // а за Caddy порт вообще может быть другим.
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{addr}");
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    Ok(())
}

async fn cache_headers(request: Request, next: middleware::Next) -> Response {
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;
    let cache_value = if path.starts_with("/admin/assets/") || path.starts_with("/uploads/") {
        Some("public, max-age=31536000, immutable")
    } else if path == "/static/style.css" {
        Some("public, max-age=3600")
    } else if path == "/static/favicon.ico"
        || path == "/favicon.ico"
        || path == "/robots.txt"
        || path == "/sitemap.xml"
    {
        Some("public, max-age=86400")
    } else {
        let is_html = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.starts_with("text/html"))
            .unwrap_or(false);
        is_html.then_some("no-cache")
    };
    if let Some(value) = cache_value {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static(value));
    }
    response
}

/// Корректная остановка: podman/systemd шлют SIGTERM и ждут 10 секунд,
/// после чего убивают процесс SIGKILL. Без этого обработчика каждый
/// рестарт — это 10 секунд простоя и обрыв запросов.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl-c");
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

async fn favicon() -> Response {
    ([(header::CONTENT_TYPE, "image/x-icon")], FAVICON).into_response()
}

async fn robots_txt(State(app): State<App>) -> Response {
    let body = format!(
        "User-agent: *\nAllow: /\nDisallow: /admin\nDisallow: /api/\nDisallow: /search\nSitemap: {}/sitemap.xml\n",
        app.public_origin
    );
    (
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        body,
    )
        .into_response()
}

async fn sitemap_xml(State(app): State<App>) -> Result<Response, AppError> {
    let items = articles::archive(&app.pool).await?;
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for path in ["/", "/archive"] {
        xml.push_str(&format!(
            "<url><loc>{}{}</loc></url>\n",
            xml_escape(&app.public_origin),
            path
        ));
    }
    for (_, slug) in articles::RUBRICS {
        xml.push_str(&format!(
            "<url><loc>{}/rubric/{slug}</loc></url>\n",
            xml_escape(&app.public_origin)
        ));
    }
    for item in items {
        let lastmod = item
            .published_at
            .map(|date| format!("<lastmod>{date}</lastmod>"))
            .unwrap_or_default();
        xml.push_str(&format!(
            "<url><loc>{}/{}</loc>{lastmod}</url>\n",
            xml_escape(&app.public_origin),
            xml_escape(&item.slug)
        ));
    }
    xml.push_str("</urlset>\n");
    Ok((
        [
            (header::CONTENT_TYPE, "application/xml; charset=utf-8"),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        xml,
    )
        .into_response())
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[derive(serde::Serialize)]
struct PageMeta {
    description: String,
    canonical: String,
    og_title: String,
    og_description: String,
    og_type: String,
    og_url: String,
    og_image: Option<String>,
    robots: String,
}

fn page_meta(
    app: &App,
    path: &str,
    title: &str,
    description: &str,
    og_type: &str,
    image: Option<String>,
    robots: &str,
) -> PageMeta {
    let canonical = format!("{}{}", app.public_origin, path);
    let og_image = image.map(|src| {
        if src.starts_with("http://") || src.starts_with("https://") {
            src
        } else {
            format!("{}{}", app.public_origin, src)
        }
    });
    PageMeta {
        description: description.to_string(),
        canonical: canonical.clone(),
        og_title: title.to_string(),
        og_description: description.to_string(),
        og_type: og_type.to_string(),
        og_url: canonical,
        og_image,
        robots: robots.to_string(),
    }
}

#[derive(serde::Serialize)]
struct IndexCtx {
    today: String,
    meta: PageMeta,
    hero: Option<articles::ArticleSummary>,
    cards: Vec<articles::ArticleSummary>,
}

/// Главная: первый материал крупно, следующие три — карточками.
async fn index(State(app): State<App>) -> Result<Html<String>, AppError> {
    let today = today_ru(&app.pool).await?;
    let mut list = articles::published_summaries(&app.pool, 4).await?;
    let cards = list.split_off(list.len().min(1));
    let hero = list.pop();
    let meta = page_meta(
        &app,
        "/",
        "Vivec — школьная газета",
        "Новости, репортажи и мнения школьной редакции Vivec.",
        "website",
        None,
        "index,follow",
    );

    let ctx = minijinja::Value::from(minijinja::value::Serde(IndexCtx {
        today,
        meta,
        hero,
        cards,
    }));
    let html = app.tpl.get_template("index.html.jinja")?.render(ctx)?;
    Ok(Html(html))
}

/// Контекст страницы статьи.
#[derive(serde::Serialize)]
struct ArticleCtx<'a> {
    today: String,
    meta: PageMeta,
    article: &'a articles::Article,
}

/// Страница статьи по slug. Черновик и несуществующий slug — одна и та же 404.
async fn article(State(app): State<App>, Path(slug): Path<String>) -> Result<Response, AppError> {
    match articles::by_slug(&app.pool, &slug).await? {
        Some(article) => {
            let today = today_ru(&app.pool).await?;
            let description = if article.lead.is_empty() {
                article.title.clone()
            } else {
                article.lead.clone()
            };
            let meta = page_meta(
                &app,
                &format!("/{}", article.slug),
                &format!("{} — Vivec", article.title),
                &description,
                "article",
                article.og_image.clone(),
                "index,follow",
            );
            let ctx = minijinja::Value::from(minijinja::value::Serde(ArticleCtx {
                today,
                meta,
                article: &article,
            }));
            let html = app.tpl.get_template("article.html.jinja")?.render(ctx)?;
            Ok(Html(html).into_response())
        }
        None => Ok(not_found(State(app)).await),
    }
}

/// Контекст страницы рубрики.
#[derive(serde::Serialize)]
struct RubricCtx {
    today: String,
    meta: PageMeta,
    rubric: &'static str,
    slug: String,
    articles: Vec<articles::ArticleSummary>,
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

    let today = today_ru(&app.pool).await?;
    let list = articles::by_rubric(&app.pool, name, 100).await?;
    let description = format!("Материалы рубрики «{name}» школьной газеты Vivec.");
    let meta = page_meta(
        &app,
        &format!("/rubric/{rubric_slug}"),
        &format!("{name} — Vivec"),
        &description,
        "website",
        None,
        "index,follow",
    );
    let ctx = minijinja::Value::from(minijinja::value::Serde(RubricCtx {
        today,
        meta,
        rubric: name,
        slug: rubric_slug,
        articles: list,
    }));
    let html = app.tpl.get_template("rubric.html.jinja")?.render(ctx)?;
    Ok(Html(html).into_response())
}

#[derive(serde::Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

#[derive(serde::Serialize)]
struct SearchCtx<'a> {
    today: String,
    meta: PageMeta,
    query: &'a str,
    articles: Vec<articles::ArticleSummary>,
}

async fn search(
    State(app): State<App>,
    Query(params): Query<SearchQuery>,
) -> Result<Html<String>, AppError> {
    let query = params.q.unwrap_or_default();
    let query = query.trim();
    let today = today_ru(&app.pool).await?;
    let found = if query.is_empty() {
        Vec::new()
    } else {
        articles::search(&app.pool, query).await?
    };
    let meta = page_meta(
        &app,
        "/search",
        "Поиск — Vivec",
        "Поиск материалов школьной газеты Vivec.",
        "website",
        None,
        "noindex,follow",
    );
    let ctx = minijinja::Value::from(minijinja::value::Serde(SearchCtx {
        today,
        meta,
        query,
        articles: found,
    }));
    let html = app.tpl.get_template("search.html.jinja")?.render(ctx)?;
    Ok(Html(html))
}

#[derive(serde::Serialize)]
struct ArchiveCtx {
    today: String,
    meta: PageMeta,
    articles: Vec<articles::ArticleSummary>,
}

async fn archive(State(app): State<App>) -> Result<Html<String>, AppError> {
    let today = today_ru(&app.pool).await?;
    let items = articles::archive(&app.pool).await?;
    let meta = page_meta(
        &app,
        "/archive",
        "Все статьи — Vivec",
        "Полный архив материалов школьной газеты Vivec.",
        "website",
        None,
        "index,follow",
    );
    let ctx = minijinja::Value::from(minijinja::value::Serde(ArchiveCtx {
        today,
        meta,
        articles: items,
    }));
    let html = app.tpl.get_template("archive.html.jinja")?.render(ctx)?;
    Ok(Html(html))
}

/// Страница 404. Отдельный хендлер, а не просто код: пользователь должен
/// видеть газету, а не белую строку «404».
async fn not_found(State(app): State<App>) -> Response {
    #[derive(serde::Serialize)]
    struct NotFoundCtx {
        today: String,
        meta: PageMeta,
    }

    let today = match today_ru(&app.pool).await {
        Ok(today) => today,
        Err(error) => {
            tracing::error!("today_ru error: {error}");
            String::new()
        }
    };
    let meta = page_meta(
        &app,
        "/404",
        "Страница не найдена — Vivec",
        "Запрошенная страница не найдена.",
        "website",
        None,
        "noindex,follow",
    );
    let ctx = minijinja::Value::from(minijinja::value::Serde(NotFoundCtx { today, meta }));
    match app
        .tpl
        .get_template("404.html.jinja")
        .and_then(|t| t.render(ctx))
    {
        Ok(html) => (StatusCode::NOT_FOUND, Html(html)).into_response(),
        Err(e) => {
            tracing::error!("404 template error: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "500").into_response()
        }
    }
}

async fn today_ru(pool: &SqlitePool) -> sqlx::Result<String> {
    let row = sqlx::query(
        "SELECT date('now', '+3 hours') AS iso,
                strftime('%w', 'now', '+3 hours') AS weekday",
    )
    .fetch_one(pool)
    .await?;
    let iso: String = sqlx::Row::try_get(&row, "iso")?;
    let weekday: String = sqlx::Row::try_get(&row, "weekday")?;
    let weekday = match weekday.as_str() {
        "0" => "воскресенье",
        "1" => "понедельник",
        "2" => "вторник",
        "3" => "среда",
        "4" => "четверг",
        "5" => "пятница",
        _ => "суббота",
    };
    let mut parts = iso.split('-');
    let year = parts.next().unwrap_or("");
    let month = parts.next().and_then(|value| value.parse::<usize>().ok());
    let day = parts.next().and_then(|value| value.parse::<u32>().ok());
    let formatted = match (month, day) {
        (Some(month), Some(day)) => {
            const MONTHS: [&str; 12] = [
                "января",
                "февраля",
                "марта",
                "апреля",
                "мая",
                "июня",
                "июля",
                "августа",
                "сентября",
                "октября",
                "ноября",
                "декабря",
            ];
            let name = MONTHS.get(month.wrapping_sub(1)).copied().unwrap_or("");
            format!("{weekday}, {day} {name} {year}")
        }
        _ => String::new(),
    };
    Ok(formatted.to_uppercase())
}
