use serde::Serialize;
use sqlx::{Row, sqlite::SqlitePool};

use crate::{markdown, slug};

/// Рубрики газеты. Список фиксированный: по нему строятся ссылки в шапке,
/// и он же отсекает случайные slug-и — `/rubric/что-угодно` не должен
/// открывать пустую страницу с кодом 200.
pub const RUBRICS: [(&str, &str); 4] = [
    ("Школа", "shkola"),
    ("Спорт", "sport"),
    ("Культура", "kultura"),
    ("Мнения", "mneniya"),
];

pub const SEARCH_LIMIT: usize = 20;

/// Русское имя рубрики по её slug из адреса.
pub fn rubric_name(rubric_slug: &str) -> Option<&'static str> {
    RUBRICS
        .iter()
        .find(|(_, s)| *s == rubric_slug)
        .map(|(name, _)| *name)
}

/// Статья в том виде, в каком её видят шаблоны.
/// `Serialize` нужен minijinja, чтобы положить статью в контекст шаблона.
#[derive(Serialize)]
pub struct ArticleSummary {
    pub slug: String,
    pub title: String,
    pub rubric: String,
    pub rubric_slug: String,
    pub author: String,
    pub lead: String,
    pub date: String,
    pub published_at: Option<String>,
}

#[derive(Serialize)]
pub struct Article {
    pub slug: String,
    pub title: String,
    pub rubric: String,
    /// Транслит рубрики для ссылки: «Школа» → «shkola».
    pub rubric_slug: String,
    pub author: String,
    pub lead: String,
    /// Тело статьи, уже превращённое из markdown в безопасный HTML.
    pub body_html: String,
    pub date: String,
    pub published_at: Option<String>,
    pub og_image: Option<String>,
}

/// Опубликованные карточки без полного Markdown: главная не должна
/// рендерить и санитайзить весь архив на каждый запрос.
pub async fn published_summaries(
    pool: &SqlitePool,
    limit: usize,
) -> sqlx::Result<Vec<ArticleSummary>> {
    let rows = sqlx::query(
        "SELECT slug, title, rubric, author, lead, published_at
           FROM articles
          WHERE published = 1
          ORDER BY published_at DESC, id DESC
          LIMIT ?",
    )
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(to_summary).collect()
}

pub async fn archive(pool: &SqlitePool) -> sqlx::Result<Vec<ArticleSummary>> {
    let rows = sqlx::query(
        "SELECT slug, title, rubric, author, lead, published_at
           FROM articles
          WHERE published = 1
          ORDER BY published_at DESC, id DESC",
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(to_summary).collect()
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

pub async fn by_rubric(
    pool: &SqlitePool,
    rubric: &str,
    limit: usize,
) -> sqlx::Result<Vec<ArticleSummary>> {
    let rows = sqlx::query(
        "SELECT slug, title, rubric, author, lead, published_at
           FROM articles
          WHERE published = 1 AND rubric = ?
          ORDER BY published_at DESC, id DESC
          LIMIT ?",
    )
    .bind(rubric)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(to_summary).collect()
}

/// Поиск по опубликованным статьям без SQLite `NOCASE`: он не меняет
/// регистр кириллицы. Для школьного объёма фильтрация в Rust предсказуемее.
pub async fn search(pool: &SqlitePool, query: &str) -> sqlx::Result<Vec<ArticleSummary>> {
    let needle = query.to_lowercase();
    let rows = sqlx::query(
        "SELECT slug, title, rubric, author, lead, published_at
           FROM articles
          WHERE published = 1
          ORDER BY published_at DESC, id DESC",
    )
    .fetch_all(pool)
    .await?;

    let mut found = Vec::new();
    for row in rows {
        let title: String = row.try_get("title")?;
        let author: String = row.try_get("author")?;
        let lead: String = row.try_get("lead")?;
        let matches = [title, author, lead]
            .iter()
            .any(|value| value.to_lowercase().contains(&needle));
        if matches {
            found.push(to_summary(row)?);
        }
        if found.len() == SEARCH_LIMIT {
            break;
        }
    }
    Ok(found)
}

fn to_summary(row: sqlx::sqlite::SqliteRow) -> sqlx::Result<ArticleSummary> {
    let published_at: Option<String> = row.try_get("published_at")?;
    let rubric: String = row.try_get("rubric")?;
    Ok(ArticleSummary {
        slug: row.try_get("slug")?,
        title: row.try_get("title")?,
        rubric_slug: slug::translit(&rubric),
        rubric,
        author: row.try_get("author")?,
        lead: row.try_get("lead")?,
        date: ru_date(published_at.as_deref().unwrap_or("")),
        published_at,
    })
}

/// Разбор строки базы. `try_get` вместо derive: одно место, где видно
/// имена колонок и типы, без макросов.
fn to_article(row: sqlx::sqlite::SqliteRow) -> sqlx::Result<Article> {
    let body: String = row.try_get("body_md")?;
    let published_at: Option<String> = row.try_get("published_at")?;
    let rubric: String = row.try_get("rubric")?;

    Ok(Article {
        slug: row.try_get("slug")?,
        title: row.try_get("title")?,
        rubric_slug: slug::translit(&rubric),
        rubric,
        author: row.try_get("author")?,
        lead: row.try_get("lead")?,
        body_html: markdown::to_html(&body),
        date: ru_date(published_at.as_deref().unwrap_or("")),
        published_at,
        og_image: markdown::first_image(&body),
    })
}

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
