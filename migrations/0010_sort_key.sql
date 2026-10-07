-- 同じ日付の記事を並べる順の鍵（ADR 0059）。ファイル名から取ったもの（UUIDv7 か、ファイル名）。
-- article は frontmatter の slug を URL にできるので、slug の順では作った順に並ばない。
ALTER TABLE posts ADD COLUMN sort_key TEXT;
ALTER TABLE articles ADD COLUMN sort_key TEXT;
