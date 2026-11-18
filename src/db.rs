use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};

/// Схема лежит в репозитории и вкомпилирована в бинарник: файл миграции
/// не может «потеряться» при деплое.
const SCHEMA: &str = include_str!("../migrations/0001_init.sql");
const ADMIN_SCHEMA: &str = include_str!("../migrations/0002_admin.sql");
const RUBRIC_INDEX: &str = include_str!("../migrations/0003_rubric_index.sql");
const DEMO_SEED: &str = include_str!("../migrations/seed.sql");

/// Открывает (при необходимости создаёт) файл базы и применяет схему.
/// Путь берётся из `DATA_DIR`, по умолчанию `data` — как в Containerfile.
pub async fn open() -> anyhow::Result<SqlitePool> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;

    let url = format!("sqlite://{dir}/vivec.db");
    let opts = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(true)
        // WAL: читатели не блокируют писателя, что для «список + админка» важно.
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePool::connect_with(opts).await?;
    sqlx::raw_sql(SCHEMA).execute(&pool).await?;
    sqlx::raw_sql(ADMIN_SCHEMA).execute(&pool).await?;
    sqlx::raw_sql(RUBRIC_INDEX).execute(&pool).await?;
    seed_demo_if_empty(&pool).await?;
    tracing::info!("база готова: {dir}/vivec.db");
    Ok(pool)
}

pub fn data_dir() -> String {
    std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string())
}

async fn seed_demo_if_empty(pool: &SqlitePool) -> sqlx::Result<()> {
    let enabled = std::env::var("VIVEC_SEED_DEMO")
        .map(|value| value == "true" || value == "1")
        .unwrap_or(false);
    if !enabled {
        return Ok(());
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM articles")
        .fetch_one(pool)
        .await?;
    if count == 0 {
        sqlx::raw_sql(DEMO_SEED).execute(pool).await?;
        tracing::info!("добавлены демонстрационные статьи");
    }
    Ok(())
}
