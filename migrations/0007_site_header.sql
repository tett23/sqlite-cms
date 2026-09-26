-- ヘッダの Markdown（ADR 0043）。content/header.md に site.toml の値を埋め込んだもの。なければ NULL（既定のヘッダ）。
ALTER TABLE site ADD COLUMN header_md TEXT;
