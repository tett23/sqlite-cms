import initSqlJs from "sql.js";
import { beforeAll, describe, expect, it } from "vitest";
import { getArticle, getCategory, getLinkCardImages, listCategories, getPage, getPost, getSite, listAll, listArticles, listPosts, tagsOf } from "./db";
import { SqliteFile } from "./sqlite";

let db: SqliteFile;

beforeAll(async () => {
  const SQL = await initSqlJs();
  const sql = new SQL.Database();
  const migrations = import.meta.glob<string>("../../migrations/*.sql", {
    query: "?raw",
    import: "default",
    eager: true,
  });
  const files = Object.keys(migrations).sort();
  expect(files.length).toBeGreaterThan(0);
  for (const file of files) {
    sql.exec(migrations[file]);
  }

  sql.run("INSERT INTO posts (slug, title, published_at, body_md) VALUES (?, ?, ?, ?)", ["old", "古い記事", "2026-01-01", "**old**"]);
  sql.run("INSERT INTO posts (slug, title, published_at, body_md) VALUES (?, ?, ?, ?)", ["new", "新しい記事", "2026-09-17", "new"]);
  sql.run("INSERT INTO articles (slug, title, published_at, updated_at, description, body_md) VALUES (?, ?, ?, ?, ?, ?)", [
    "long",
    "長い読み物",
    "2026-09-15",
    "2026-09-16",
    "要約。",
    "| a |\n|---|\n| 1 |",
  ]);
  sql.run("INSERT INTO pages VALUES (?, ?, ?)", ["about", "自己紹介", "# about"]);
  sql.run("INSERT INTO site VALUES (1, ?, ?, ?, ?, ?, ?, ?)", [
    "記事置き場",
    null,
    "CC0 1.0",
    "https://creativecommons.org/publicdomain/zero/1.0/",
    "トップの**導入**。",
    "組版の記事",
    "[記事置き場](/)",
  ]);
  sql.run("INSERT INTO link_cards VALUES (?, ?)", ["https://example.com/", "/link-cards/0123456789abcdef.png"]);
  // タグ（ADR 0048）。position の順に返すことを確かめるため、逆の順に入れる。
  sql.run("INSERT INTO tags VALUES ('article', 'long', 1, 'SQLite')");
  sql.run("INSERT INTO tags VALUES ('article', 'long', 0, '組版')");
  sql.run("INSERT INTO tags VALUES ('post', 'new', 0, '日記')");
  // sql.js で作った DB を書き出し、ページの表示と同じく自前の読み手で開く（ADR 0047）。
  db = new SqliteFile(sql.export());
  sql.close();
});

describe("getLinkCardImages", () => {
  it("リンクカードの URL と画像のパスを返し、同じ DB では同じ Map を使い回す", () => {
    const images = getLinkCardImages(db);
    expect([...images]).toEqual([["https://example.com/", "/link-cards/0123456789abcdef.png"]]);
    expect(getLinkCardImages(db)).toBe(images);
  });
});

describe("同じ日付の記事の並び（ADR 0059）", () => {
  it("並べる順の鍵（sort_key）の降順に並べ、slug の順には並べない", async () => {
    const SQL = await initSqlJs();
    const sql = new SQL.Database();
    const migrations = import.meta.glob<string>("../../migrations/*.sql", { query: "?raw", import: "default", eager: true });
    for (const file of Object.keys(migrations).sort()) sql.exec(migrations[file]);
    // slug の順（a < z）と、鍵の順（2 < 1 の逆）を逆にする。
    sql.run("INSERT INTO articles (slug, title, published_at, body_md, sort_key) VALUES ('aaa', '後から', '2026-09-26', '', '0002')");
    sql.run("INSERT INTO articles (slug, title, published_at, body_md, sort_key) VALUES ('zzz', '先に', '2026-09-26', '', '0001')");
    sql.run("INSERT INTO posts (slug, title, published_at, body_md, sort_key) VALUES ('p', 'メモ', '2026-09-26', '', 'p')");
    sql.run("INSERT INTO site (id, title) VALUES (1, 't')");
    const db = new SqliteFile(sql.export());
    sql.close();
    expect(listArticles(db).map((a) => a.slug)).toEqual(["aaa", "zzz"]);
    expect(listAll(db).map((e) => e.slug)).toEqual(["aaa", "zzz", "p"]);
  });
});

describe("カテゴリ（ADR 0060）", () => {
  async function dbWith(statements: string[], withMigrations = true): Promise<SqliteFile> {
    const SQL = await initSqlJs();
    const sql = new SQL.Database();
    if (withMigrations) {
      const migrations = import.meta.glob<string>("../../migrations/*.sql", { query: "?raw", import: "default", eager: true });
      for (const file of Object.keys(migrations).sort()) sql.exec(migrations[file]);
    }
    for (const statement of statements) sql.run(statement);
    const file = new SqliteFile(sql.export());
    sql.close();
    return file;
  }

  it("カテゴリを並び順に返し、article の数を数え、カテゴリで article を絞る", async () => {
    const db = await dbWith([
      "INSERT INTO categories VALUES ('samples', '見本', NULL, '', 1)",
      "INSERT INTO categories VALUES ('howto', '使い方', '書き方の案内', '# 案内', 0)",
      "INSERT INTO articles (slug, title, published_at, body_md, category) VALUES ('a', 'A', '2026-09-01', '', 'howto')",
      "INSERT INTO articles (slug, title, published_at, body_md, category) VALUES ('b', 'B', '2026-09-02', '', 'howto')",
      "INSERT INTO articles (slug, title, published_at, body_md) VALUES ('c', 'C', '2026-09-03', '')",
    ]);
    expect(listCategories(db)).toEqual([
      { slug: "howto", title: "使い方", description: "書き方の案内", bodyMd: "# 案内", articleCount: 2 },
      { slug: "samples", title: "見本", description: null, bodyMd: "", articleCount: 0 },
    ]);
    expect(listCategories(db)).toBe(listCategories(db));
    expect(getCategory(db, "howto")?.title).toBe("使い方");
    expect(getCategory(db, "missing")).toBeNull();
    expect(listArticles(db, "howto").map((a) => a.slug)).toEqual(["b", "a"]);
    expect(listArticles(db, "samples")).toEqual([]);
    expect(listArticles(db).map((a) => a.slug)).toEqual(["c", "b", "a"]);
    expect(getArticle(db, "a")?.category).toEqual({ slug: "howto", title: "使い方" });
    expect(getArticle(db, "c")?.category).toBeNull();
  });

  it("カテゴリの表のない DB では、カテゴリなしとして扱う", async () => {
    const db = await dbWith(["CREATE TABLE articles (slug TEXT PRIMARY KEY, title TEXT, published_at TEXT, body_md TEXT)", "INSERT INTO articles VALUES ('a', 'A', '2026-09-01', '')"], false);
    expect(listCategories(db)).toEqual([]);
    expect(getArticle(db, "a")?.category).toBeNull();
  });
});

describe("listPosts / getPost", () => {
  it("日付降順で返す", () => {
    expect(listPosts(db).map((p) => p.slug)).toEqual(["new", "old"]);
  });

  it("slug で本文込みの post を返す", () => {
    expect(getPost(db, "old")).toEqual({
      slug: "old",
      title: "古い記事",
      publishedAt: "2026-01-01",
      bodyMd: "**old**",
      tags: [],
    });
  });

  it("存在しない slug は null を返す", () => {
    expect(getPost(db, "missing")).toBeNull();
  });
});

describe("listArticles / getArticle", () => {
  it("一覧に description が含まれる", () => {
    expect(listArticles(db)).toEqual([
      { slug: "long", title: "長い読み物", publishedAt: "2026-09-15", description: "要約。" },
    ]);
  });

  it("本文と改稿日込みの article を返す", () => {
    expect(getArticle(db, "long")).toEqual({
      slug: "long",
      title: "長い読み物",
      publishedAt: "2026-09-15",
      updatedAt: "2026-09-16",
      description: "要約。",
      bodyMd: "| a |\n|---|\n| 1 |",
      tags: ["組版", "SQLite"],
      category: null,
    });
  });
});

describe("getPage", () => {
  it("slug でページを返す", () => {
    expect(getPage(db, "about")).toEqual({ slug: "about", title: "自己紹介", bodyMd: "# about" });
  });

  it("存在しない slug は null を返す", () => {
    expect(getPage(db, "missing")).toBeNull();
  });
});

describe("getSite", () => {
  it("サイトのメタデータとトップページの本文を返す", () => {
    expect(getSite(db)).toEqual({
      title: "記事置き場",
      author: null,
      license: { name: "CC0 1.0", url: "https://creativecommons.org/publicdomain/zero/1.0/" },
      homeMd: "トップの**導入**。",
      description: "組版の記事",
      headerMd: "[記事置き場](/)",
    });
  });

  it("ライセンスとヘッダがなければ null を返す", async () => {
    const SQL = await initSqlJs();
    const bare = new SQL.Database();
    bare.run(
      "CREATE TABLE site (id INTEGER, title TEXT, author TEXT, license_name TEXT, license_url TEXT, home_md TEXT, description TEXT, header_md TEXT)",
    );
    bare.run("INSERT INTO site VALUES (1, 't', NULL, NULL, NULL, NULL, 'd', NULL)");
    const file = new SqliteFile(bare.export());
    expect(getSite(file).license).toBeNull();
    expect(getSite(file).headerMd).toBeNull();
    bare.close();
  });
});

describe("listAll（ADR 0033）", () => {
  it("post と article を、日付の新しい順にまとめる", () => {
    expect(listAll(db).map((entry) => [entry.kind, entry.slug, entry.publishedAt, entry.description])).toEqual([
      ["post", "new", "2026-09-17", null],
      ["article", "long", "2026-09-15", "要約。"],
      ["post", "old", "2026-01-01", null],
    ]);
  });
});

describe("タグ（ADR 0048）", () => {
  it("記事と post のタグを、書いた順に返す。タグがなければ空", () => {
    expect(getArticle(db, "long")?.tags).toEqual(["組版", "SQLite"]);
    expect(getPost(db, "new")?.tags).toEqual(["日記"]);
    expect(getPost(db, "old")?.tags).toEqual([]);
    // 同じ slug でも、種類が違えば別のタグ。
    expect(tagsOf(db, "post", "long")).toEqual([]);
  });

  it("タグの表のない DB では、空を返す", async () => {
    const SQL = await initSqlJs();
    const old = new SQL.Database();
    old.run("CREATE TABLE posts (slug TEXT PRIMARY KEY, title TEXT, published_at TEXT, body_md TEXT)");
    old.run("INSERT INTO posts VALUES ('p', 't', '2026-01-01', '本文')");
    expect(getPost(new SqliteFile(old.export()), "p")?.tags).toEqual([]);
    old.close();
  });
});
