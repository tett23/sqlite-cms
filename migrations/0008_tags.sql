-- 記事のタグ（ADR 0048）。kind は post か article、position は frontmatter に書いた順（0 から）。
-- タグは、先頭の # を付けずに入れる。
CREATE TABLE tags (
  kind     TEXT NOT NULL,
  slug     TEXT NOT NULL,
  position INTEGER NOT NULL,
  tag      TEXT NOT NULL,
  PRIMARY KEY (kind, slug, position)
);
