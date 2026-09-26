ALTER TABLE articles RENAME TO posts;

CREATE TABLE articles (
  slug         TEXT PRIMARY KEY,
  title        TEXT NOT NULL,
  published_at TEXT NOT NULL,
  updated_at   TEXT,
  description  TEXT,
  body_html    TEXT NOT NULL
);

CREATE TABLE pages (
  slug      TEXT PRIMARY KEY,
  title     TEXT NOT NULL,
  body_html TEXT NOT NULL
);
