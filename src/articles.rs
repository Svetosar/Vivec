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

/// Статьи одной рубрики. В базе рубрика записана по-русски, в адресе —
/// транслит, поэтому фильтруем в Rust: статей у школьной газеты немного,
/// лишний SELECT по всем строкам дешевле, чем отдельная колонка со slug.
pub async fn by_rubric(pool: &SqlitePool, rubric_slug: &str) -> sqlx::Result<Vec<Article>> {
    let mut list = published(pool).await?;
    list.retain(|a| a.rubric_slug == rubric_slug);
    Ok(list)
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
