use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, sqlite::SqlitePool};

use crate::{App, articles::RUBRICS, error::AppError, slug, uploads};

const MAX_TITLE: usize = 200;
const MAX_SLUG: usize = 120;
const MAX_AUTHOR: usize = 100;
const MAX_LEAD: usize = 500;
const MAX_BODY: usize = 200_000;
const RESERVED_SLUGS: [&str; 9] = [
    "admin",
    "api",
    "uploads",
    "static",
    "search",
    "healthz",
    "archive",
    "robots-txt",
    "sitemap-xml",
];

#[derive(Serialize)]
pub struct ArticleSummary {
    id: i64,
    slug: String,
    title: String,
    rubric: String,
    author: String,
    published: bool,
    published_at: Option<String>,
    created_at: String,
}

#[derive(Serialize)]
pub struct AdminArticle {
    id: i64,
    slug: String,
    title: String,
    rubric: String,
    author: String,
    lead: String,
    body_md: String,
    published: bool,
    published_at: Option<String>,
    created_at: String,
}

#[derive(Deserialize)]
pub struct ArticleInput {
    title: String,
    slug: String,
    rubric: String,
    author: String,
    lead: String,
    body_md: String,
}

#[derive(Deserialize)]
pub struct ListQuery {
    status: Option<String>,
    q: Option<String>,
}

#[derive(Serialize)]
pub struct ArticleList {
    items: Vec<ArticleSummary>,
}

pub async fn list(
    State(app): State<App>,
    Query(query): Query<ListQuery>,
) -> Result<Json<ArticleList>, AppError> {
    let status = query.status.as_deref().unwrap_or("all");
    if !matches!(status, "all" | "draft" | "published") {
        return Err(AppError::BadRequest("неизвестный статус статьи"));
    }

    let rows = sqlx::query(
        "SELECT id, slug, title, rubric, author, published, published_at, created_at
           FROM articles
          ORDER BY created_at DESC, id DESC",
    )
    .fetch_all(&app.pool)
    .await?;

    let mut items = rows
        .into_iter()
        .map(to_summary)
        .collect::<sqlx::Result<Vec<_>>>()?;

    if status == "draft" {
        items.retain(|article| !article.published);
    } else if status == "published" {
        items.retain(|article| article.published);
    }

    if let Some(needle) = query.q.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        let needle = needle.to_lowercase();
        items.retain(|article| {
            article.title.to_lowercase().contains(&needle)
                || article.slug.to_lowercase().contains(&needle)
                || article.author.to_lowercase().contains(&needle)
        });
    }

    Ok(Json(ArticleList { items }))
}

pub async fn get(
    State(app): State<App>,
    Path(id): Path<i64>,
) -> Result<Json<AdminArticle>, AppError> {
    Ok(Json(get_by_id(&app.pool, id).await?))
}

pub async fn create(
    State(app): State<App>,
    Json(input): Json<ArticleInput>,
) -> Result<impl IntoResponse, AppError> {
    let input = validate(input)?;
    let result = sqlx::query(
        "INSERT INTO articles (slug, title, rubric, author, lead, body_md)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&input.slug)
    .bind(&input.title)
    .bind(&input.rubric)
    .bind(&input.author)
    .bind(&input.lead)
    .bind(&input.body_md)
    .execute(&app.pool)
    .await;

    let id = match result {
        Ok(result) => result.last_insert_rowid(),
        Err(error) if is_unique_violation(&error) => {
            return Err(AppError::Conflict("такой slug уже существует"));
        }
        Err(error) => return Err(error.into()),
    };

    let article = get_by_id(&app.pool, id).await?;
    Ok((StatusCode::CREATED, Json(article)))
}

pub async fn update(
    State(app): State<App>,
    Path(id): Path<i64>,
    Json(input): Json<ArticleInput>,
) -> Result<Json<AdminArticle>, AppError> {
    let input = validate(input)?;
    let existing = get_by_id(&app.pool, id).await?;

    let result = sqlx::query(
        "UPDATE articles
            SET slug = ?, title = ?, rubric = ?, author = ?, lead = ?, body_md = ?
          WHERE id = ?",
    )
    .bind(&input.slug)
    .bind(&input.title)
    .bind(&input.rubric)
    .bind(&input.author)
    .bind(&input.lead)
    .bind(&input.body_md)
    .bind(id)
    .execute(&app.pool)
    .await;

    match result {
        Ok(_) => {}
        Err(error) if is_unique_violation(&error) => {
            return Err(AppError::Conflict("такой slug уже существует"));
        }
        Err(error) => return Err(error.into()),
    }

    uploads::cleanup_unused(&app.pool, &existing.body_md).await;
    Ok(Json(get_by_id(&app.pool, id).await?))
}

pub async fn publish(
    State(app): State<App>,
    Path(id): Path<i64>,
) -> Result<Json<AdminArticle>, AppError> {
    let result = sqlx::query(
        "UPDATE articles
            SET published = 1, published_at = date('now', '+3 hours')
          WHERE id = ?",
    )
    .bind(id)
    .execute(&app.pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(get_by_id(&app.pool, id).await?))
}

pub async fn unpublish(
    State(app): State<App>,
    Path(id): Path<i64>,
) -> Result<Json<AdminArticle>, AppError> {
    let result = sqlx::query("UPDATE articles SET published = 0 WHERE id = ?")
        .bind(id)
        .execute(&app.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(get_by_id(&app.pool, id).await?))
}

pub async fn delete(State(app): State<App>, Path(id): Path<i64>) -> Result<StatusCode, AppError> {
    let existing = get_by_id(&app.pool, id).await?;
    let result = sqlx::query("DELETE FROM articles WHERE id = ?")
        .bind(id)
        .execute(&app.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    uploads::cleanup_unused(&app.pool, &existing.body_md).await;
    Ok(StatusCode::NO_CONTENT)
}

async fn get_by_id(pool: &SqlitePool, id: i64) -> Result<AdminArticle, AppError> {
    let row = sqlx::query(
        "SELECT id, slug, title, rubric, author, lead, body_md,
                published, published_at, created_at
           FROM articles
          WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(AdminArticle {
        id: row.try_get("id")?,
        slug: row.try_get("slug")?,
        title: row.try_get("title")?,
        rubric: row.try_get("rubric")?,
        author: row.try_get("author")?,
        lead: row.try_get("lead")?,
        body_md: row.try_get("body_md")?,
        published: row.try_get::<i64, _>("published")? == 1,
        published_at: row.try_get("published_at")?,
        created_at: row.try_get("created_at")?,
    })
}

fn to_summary(row: sqlx::sqlite::SqliteRow) -> sqlx::Result<ArticleSummary> {
    Ok(ArticleSummary {
        id: row.try_get("id")?,
        slug: row.try_get("slug")?,
        title: row.try_get("title")?,
        rubric: row.try_get("rubric")?,
        author: row.try_get("author")?,
        published: row.try_get::<i64, _>("published")? == 1,
        published_at: row.try_get("published_at")?,
        created_at: row.try_get("created_at")?,
    })
}

fn validate(mut input: ArticleInput) -> Result<ArticleInput, AppError> {
    input.title = input.title.trim().to_string();
    input.author = input.author.trim().to_string();
    input.lead = input.lead.trim().to_string();
    input.rubric = input.rubric.trim().to_string();

    if input.title.is_empty() || input.title.chars().count() > MAX_TITLE {
        return Err(AppError::BadRequest("заголовок: от 1 до 200 символов"));
    }
    if input.author.chars().count() > MAX_AUTHOR {
        return Err(AppError::BadRequest("автор: не больше 100 символов"));
    }
    if input.lead.chars().count() > MAX_LEAD {
        return Err(AppError::BadRequest("лид: не больше 500 символов"));
    }
    if input.body_md.len() > MAX_BODY {
        return Err(AppError::BadRequest("текст статьи больше 200 КБ"));
    }
    if !RUBRICS.iter().any(|(name, _)| *name == input.rubric) {
        return Err(AppError::BadRequest("неизвестная рубрика"));
    }

    input.slug = if input.slug.trim().is_empty() {
        slug::translit(&input.title)
    } else {
        slug::translit(input.slug.trim())
    };
    if input.slug.is_empty() || input.slug.chars().count() > MAX_SLUG {
        return Err(AppError::BadRequest("slug: от 1 до 120 символов"));
    }
    if RESERVED_SLUGS.contains(&input.slug.as_str()) {
        return Err(AppError::BadRequest("этот slug занят системным маршрутом"));
    }

    Ok(input)
}

fn is_unique_violation(error: &sqlx::Error) -> bool {
    error
        .as_database_error()
        .map(|error| error.is_unique_violation())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{ArticleInput, validate};

    fn article() -> ArticleInput {
        ArticleInput {
            title: "  Новый материал  ".to_string(),
            slug: "".to_string(),
            rubric: "Школа".to_string(),
            author: "Редактор".to_string(),
            lead: "Лид".to_string(),
            body_md: "Текст".to_string(),
        }
    }

    #[test]
    fn empty_slug_is_generated_from_title() {
        let input = validate(article()).unwrap();
        assert_eq!(input.title, "Новый материал");
        assert_eq!(input.slug, "novyy-material");
    }

    #[test]
    fn unknown_rubric_is_rejected() {
        let mut input = article();
        input.rubric = "Сплетни".to_string();
        assert!(validate(input).is_err());
    }

    #[test]
    fn reserved_slug_is_rejected() {
        let mut input = article();
        input.slug = "admin".to_string();
        assert!(validate(input).is_err());
    }
}
