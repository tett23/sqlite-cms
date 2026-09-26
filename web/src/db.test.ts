import initSqlJs, { type Database } from "sql.js";
import { beforeAll, describe, expect, it } from "vitest";
import { getArticle, getLinkCardImages, getPage, getPost, getSite, listAll, listArticles, listPosts } from "./db";

let db: Database;

beforeAll(async () => {
  const SQL = await initSqlJs();
  db = new SQL.Database();
  const migrations = import.meta.glob<string>("../../migrations/*.sql", {
    query: "?raw",
    import: "default",
    eager: true,
  });
  const files = Object.keys(migrations).sort();
  expect(files.length).toBeGreaterThan(0);
  for (const file of files) {
    db.exec(migrations[file]);
  }

  db.run("INSERT INTO posts VALUES (?, ?, ?, ?)", ["old", "古い記事", "2026-01-01", "**old**"]);
  db.run("INSERT INTO posts VALUES (?, ?, ?, ?)", ["new", "新しい記事", "2026-09-17", "new"]);
  db.run("INSERT INTO articles VALUES (?, ?, ?, ?, ?, ?)", [
    "long",
    "長い読み物",
    "2026-09-15",
    "2026-09-16",
    "要約。",
    "| a |\n|---|\n| 1 |",
  ]);
  db.run("INSERT INTO pages VALUES (?, ?, ?)", ["about", "自己紹介", "# about"]);
  db.run("INSERT INTO site VALUES (1, ?, ?, ?, ?, ?, ?)", [
    "記事置き場",
    null,
    "CC0 1.0",
    "https://creativecommons.org/publicdomain/zero/1.0/",
    "トップの**導入**。",
    "組版の記事",
  ]);
  db.run("INSERT INTO link_cards VALUES (?, ?)", ["https://example.com/", "/link-cards/0123456789abcdef.png"]);
});

describe("getLinkCardImages", () => {
  it("リンクカードの URL と画像のパスを返し、同じ DB では同じ Map を使い回す", () => {
    const images = getLinkCardImages(db);
    expect([...images]).toEqual([["https://example.com/", "/link-cards/0123456789abcdef.png"]]);
    expect(getLinkCardImages(db)).toBe(images);
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
    });
  });

  it("ライセンスがなければ null を返す", async () => {
    const SQL = await initSqlJs();
    const bare = new SQL.Database();
    bare.run(
      "CREATE TABLE site (id INTEGER, title TEXT, author TEXT, license_name TEXT, license_url TEXT, home_md TEXT, description TEXT)",
    );
    bare.run("INSERT INTO site VALUES (1, 't', NULL, NULL, NULL, NULL, 'd')");
    expect(getSite(bare).license).toBeNull();
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
