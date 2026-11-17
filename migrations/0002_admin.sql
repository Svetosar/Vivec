-- Админские сессии. В cookie едет случайный токен, в базе лежит только
-- его SHA-256: утечка таблицы не даёт готовых cookie.

CREATE TABLE IF NOT EXISTS sessions (
    token_hash TEXT PRIMARY KEY,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_expires
    ON sessions (expires_at);
