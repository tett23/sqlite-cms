// db.ts の一覧と取り出し（ADR 0047）が、以前の SQL の問い合わせと同じ結果になることを、乱数で作った DB で確かめる。
import initSqlJs, { type Database, type SqlJsStatic } from "sql.js";
import { beforeAll, describe, expect, it } from "vitest";
import { getArticle, getPage, getPost, listAll, listArticles, listPosts } from "./db";
import { SqliteFile } from "./sqlite";

let SQL: SqlJsStatic;

beforeAll(async () => {
  SQL = await initSqlJs();
});

function random(seed: number) {
  let x = seed >>> 0 || 1;
  const next = () => {
    x ^= x << 13;
    x >>>= 0;
    x ^= x >>> 17;
    x ^= x << 5;
    x >>>= 0;
    return x / 0x100000000;
  };
  const int = (n: number) => Math.floor(next() * n);
  const pick = <T>(items: readonly T[]): T => items[int(items.length)];
  return { int, pick };
}

// 同じ日付が多く出るよう、日付の候補を少なくする。
const DATES = ["2025-12-31", "2026-01-01", "2026-09-25", "2026-09-26", "2026-10-01"];
const SLUG_CHARS = Array.from("abcxyz019-_日本語あ");

function build(seed: number): { sql: Database; file: SqliteFile; slugs: { posts: string[]; articles: string[]; pages: string[] } } {
  const r = random(seed);
  const sql = new SQL.Database();
  const migrations = import.meta.glob<string>("../../migrations/*.sql", { query: "?raw", import: "default", eager: true });
  for (const file of Object.keys(migrations).sort()) sql.exec(migrations[file]);
  const slug = (used: Set<string>) => {
    let s = "";
    while (s === "" || used.has(s)) s = Array.from({ length: 1 + r.int(4) }, () => r.pick(SLUG_CHARS)).join("");
    used.add(s);
    return s;
  };
  const posts = new Set<string>();
  const articles = new Set<string>();
  const pages = new Set<string>();
  for (let i = 0; i < r.int(40); i++) {
    sql.run("INSERT INTO posts (slug, title, published_at, body_md) VALUES (?, ?, ?, ?)", [slug(posts), `post ${i}`, r.pick(DATES), `本文 ${i}`]);
  }
  for (let i = 0; i < r.int(40); i++) {
    sql.run("INSERT INTO articles (slug, title, published_at, updated_at, description, body_md) VALUES (?, ?, ?, ?, ?, ?)", [
      slug(articles),
      `article ${i}`,
      r.pick(DATES),
      r.int(2) === 0 ? null : r.pick(DATES),
      r.int(2) === 0 ? null : `要約 ${i}`,
      `本文 ${i}`,
    ]);
  }
  for (let i = 0; i < r.int(5); i++) sql.run("INSERT INTO pages VALUES (?, ?, ?)", [slug(pages), `page ${i}`, `本文 ${i}`]);
  return { sql, file: new SqliteFile(sql.export()), slugs: { posts: [...posts], articles: [...articles], pages: [...pages] } };
}

function rows(db: Database, query: string, params: (string | null)[] = []) {
  const stmt = db.prepare(query);
  stmt.bind(params);
  const out = [];
  while (stmt.step()) out.push(stmt.get());
  stmt.free();
  return out;
}

describe("一覧と取り出しが、以前の SQL と同じ結果になる（ADR 0047）", () => {
  it.each(Array.from({ length: 30 }, (_, i) => i + 1))("種 %d", (seed) => {
    const { sql, file, slugs } = build(seed * 104729);

    expect(listPosts(file).map((p) => [p.slug, p.title, p.publishedAt])).toEqual(
      rows(sql, "SELECT slug, title, published_at FROM posts ORDER BY published_at DESC, slug DESC"),
    );
    expect(listArticles(file).map((a) => [a.slug, a.title, a.publishedAt, a.description])).toEqual(
      rows(sql, "SELECT slug, title, published_at, description FROM articles ORDER BY published_at DESC, slug DESC"),
    );
    expect(listAll(file).map((e) => [e.kind, e.slug, e.title, e.publishedAt, e.description])).toEqual(
      rows(
        sql,
        `SELECT 'article', slug, title, published_at, description FROM articles
         UNION ALL
         SELECT 'post', slug, title, published_at, NULL FROM posts
         ORDER BY 4 DESC, 1, 2 DESC`,
      ),
    );

    for (const slug of [...slugs.posts, "存在しない"]) {
      const [row] = rows(sql, "SELECT slug, title, published_at, body_md FROM posts WHERE slug = ?", [slug]);
      const post = getPost(file, slug);
      expect(post && [post.slug, post.title, post.publishedAt, post.bodyMd]).toEqual(row ?? null);
    }
    for (const slug of [...slugs.articles, "存在しない"]) {
      const [row] = rows(sql, "SELECT slug, title, published_at, updated_at, description, body_md FROM articles WHERE slug = ?", [slug]);
      const article = getArticle(file, slug);
      expect(article && [article.slug, article.title, article.publishedAt, article.updatedAt, article.description, article.bodyMd]).toEqual(
        row ?? null,
      );
    }
    for (const slug of [...slugs.pages, "存在しない"]) {
      const [row] = rows(sql, "SELECT slug, title, body_md FROM pages WHERE slug = ?", [slug]);
      const page = getPage(file, slug);
      expect(page && [page.slug, page.title, page.bodyMd]).toEqual(row ?? null);
    }
    sql.close();
  });
});
