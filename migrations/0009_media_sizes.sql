-- 本文の画像（content/media）の大きさ（ADR 0051）。path は配信するパス（/media/…）。
-- SPA は img に width と height を付け、画像を読み込む前から場所を取って、本文がずれないようにする。
CREATE TABLE media_sizes (
  path   TEXT PRIMARY KEY,
  width  INTEGER NOT NULL,
  height INTEGER NOT NULL
);
