-- コードの埋め込み（ADR 0065）。URL だけの行の GitHub のファイルと Gist の URL と、表示するファイル。
-- 一つの URL に、表示するファイルの数だけ行を入れる（Gist は複数のファイルを持つ）。
CREATE TABLE code_embed_files (
  url         TEXT NOT NULL,
  position    INTEGER NOT NULL,
  file_name   TEXT NOT NULL,
  source_url  TEXT NOT NULL,
  start_line  INTEGER NOT NULL,
  content     TEXT NOT NULL,
  PRIMARY KEY (url, position)
);
