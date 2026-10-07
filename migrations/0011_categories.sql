-- 記事のカテゴリ（ADR 0060）。content/categories/<slug>.md から作る。URL は /<slug>（サイトの直下）。
-- position は、カテゴリの並び順（0 から）。frontmatter の order の小さい順、order のないものは後ろに title の順。
CREATE TABLE categories (
  slug        TEXT PRIMARY KEY,
  title       TEXT NOT NULL,
  description TEXT,
  body_md     TEXT NOT NULL,
  position    INTEGER
);

-- article の属するカテゴリの slug（一つだけ。なければ NULL）。
ALTER TABLE articles ADD COLUMN category TEXT;
