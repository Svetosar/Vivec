use serde::Serialize;
use sqlx::{Row, sqlite::SqlitePool};

/// Статья в том виде, в каком её видят шаблоны.
/// `Serialize` нужен minijinja, чтобы положить статью в контекст шаблона.
#[derive(Serialize)]
pub struct Article {
    pub slug: String,
    pub title: String,
    pub rubric: String,
    pub author: String,
    pub lead: String,
    /// Абзацы текста. Markdown добавится 17.11, пока режем по пустым строкам.
    pub paragraphs: Vec<String>,
    pub date: String,
}

/// Опубликованные статьи, свежие сверху. Запрос текстовый (runtime):
/// макросы `query!` запрещены правилами проекта.
pub async fn published(pool: &SqlitePool) -> sqlx::Result<Vec<Article>> {
    let rows = sqlx::query(
        "SELECT slug, title, rubric, author, lead, body_md, published_at
           FROM articles
          WHERE published = 1
          ORDER BY published_at DESC, id DESC",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(to_article).collect()
}

/// Одна статья по slug. Черновики наружу не отдаём.
pub async fn by_slug(pool: &SqlitePool, slug: &str) -> sqlx::Result<Option<Article>> {
    let row = sqlx::query(
        "SELECT slug, title, rubric, author, lead, body_md, published_at
           FROM articles
          WHERE slug = ? AND published = 1",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;

    row.map(to_article).transpose()
}

/// Разбор строки базы. `try_get` вместо derive: одно место, где видно
/// имена колонок и типы, без макросов.
fn to_article(row: sqlx::sqlite::SqliteRow) -> sqlx::Result<Article> {
    let body: String = row.try_get("body_md")?;
    let published_at: Option<String> = row.try_get("published_at")?;

    Ok(Article {
        slug: row.try_get("slug")?,
        title: row.try_get("title")?,
        rubric: row.try_get("rubric")?,
        author: row.try_get("author")?,
        lead: row.try_get("lead")?,
        paragraphs: body
            .split("\n\n")
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(String::from)
            .collect(),
        date: ru_date(published_at.as_deref().unwrap_or("")),
    })
}

const MONTHS: [&str; 12] = [
    "января", "февраля", "марта", "апреля", "мая", "июня",
    "июля", "августа", "сентября", "октября", "ноября", "декабря",
];

/// "2026-11-12" → "12 ноября". Пустая или битая дата — пустая строка:
/// лучше без даты, чем «12 undefined».
fn ru_date(iso: &str) -> String {
    let mut parts = iso.split('-');
    let _year = parts.next();
    let month = parts.next().and_then(|m| m.parse::<usize>().ok());
    let day = parts.next().and_then(|d| d.parse::<u32>().ok());

    match (month, day) {
        // "2026-11-05" → "5 ноября", а не "05 ноября": ведущий ноль тут лишний.
        (Some(m), Some(d)) => MONTHS
            .get(m.wrapping_sub(1))
            .map(|name| format!("{d} {name}"))
            .unwrap_or_default(),
        _ => String::new(),
    }
}
