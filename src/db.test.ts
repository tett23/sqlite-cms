import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import initSqlJs, { type Database } from "sql.js";
import { beforeAll, describe, expect, it } from "vitest";
import { getArticle, getPage, getPost, listArticles, listPosts } from "./db";

let db: Database;

beforeAll(async () => {
  const SQL = await initSqlJs();
  db = new SQL.Database();
  const migrationsDir = path.resolve(import.meta.dirname, "../migrations");
  for (const file of readdirSync(migrationsDir).filter((f) => f.endsWith(".sql")).sort()) {
    db.exec(readFileSync(path.join(migrationsDir, file), "utf8"));
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
