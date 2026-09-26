CREATE TABLE site (
  id           INTEGER PRIMARY KEY CHECK (id = 1),
  title        TEXT NOT NULL,
  author       TEXT,
  license_name TEXT,
  license_url  TEXT,
  home_md      TEXT
);
