CREATE INDEX IF NOT EXISTS idx_articles_rubric_published
    ON articles (rubric, published, published_at DESC);
