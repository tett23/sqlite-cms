-- リンクカードの画像（ADR 0028）。URL だけの行の URL と、配信する画像のパス。
CREATE TABLE link_cards (
  url        TEXT PRIMARY KEY,
  image_path TEXT NOT NULL
);
