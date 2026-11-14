use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};

/// Схема лежит в репозитории и вкомпилирована в бинарник: файл миграции
/// не может «потеряться» при деплое.
const SCHEMA: &str = include_str!("../migrations/0001_init.sql");

/// Открывает (при необходимости создаёт) файл базы и применяет схему.
/// Путь берётся из `DATA_DIR`, по умолчанию `data` — как в Containerfile.
pub async fn open() -> anyhow::Result<SqlitePool> {
    let dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    std::fs::create_dir_all(&dir)?;

    let url = format!("sqlite://{dir}/vivec.db");
    let opts = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(true)
        // WAL: читатели не блокируют писателя, что для «список + админка» важно.
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePool::connect_with(opts).await?;
    sqlx::raw_sql(SCHEMA).execute(&pool).await?;
    tracing::info!("база готова: {dir}/vivec.db");
    Ok(pool)
}
