CREATE TABLE articles (
  slug         TEXT PRIMARY KEY,
  title        TEXT NOT NULL,
  published_at TEXT NOT NULL,
  body_html    TEXT NOT NULL
);
