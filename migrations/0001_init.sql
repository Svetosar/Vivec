-- Схема Vivec, версия 1. Применяется при старте: CREATE IF NOT EXISTS,
-- поэтому повторный запуск ничего не ломает.
--
-- Даты — TEXT 'ГГГГ-ММ-ДД': sqlx отдаёт их строкой, форматируем в Rust.
-- chrono в типах не держим (правило проекта).

CREATE TABLE IF NOT EXISTS articles (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    slug         TEXT    NOT NULL UNIQUE,
    title        TEXT    NOT NULL,
    rubric       TEXT    NOT NULL DEFAULT 'Школа',
    author       TEXT    NOT NULL DEFAULT '',
    lead         TEXT    NOT NULL DEFAULT '',
    body_md      TEXT    NOT NULL DEFAULT '',
    published    INTEGER NOT NULL DEFAULT 0,   -- 0 — черновик, 1 — на сайте
    published_at TEXT,                          -- заполняется при публикации
    created_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Список на главной: только опубликованные, свежие сверху.
CREATE INDEX IF NOT EXISTS idx_articles_published
    ON articles (published, published_at DESC);
