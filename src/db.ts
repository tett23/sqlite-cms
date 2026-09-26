import initSqlJs, { type Database, type SqlValue } from "sql.js";
import wasmUrl from "sql.js/dist/sql-wasm.wasm?url";

export interface PostSummary {
  slug: string;
  title: string;
  publishedAt: string;
}

export interface Post extends PostSummary {
  bodyMd: string;
}

export interface ArticleSummary extends PostSummary {
  description: string | null;
}

export interface Article extends ArticleSummary {
  updatedAt: string | null;
  bodyMd: string;
}

export interface Page {
  slug: string;
  title: string;
  bodyMd: string;
}

export async function loadDb(): Promise<Database> {
  const manifestRes = await fetch("/db/manifest.json", { cache: "no-cache" });
  if (!manifestRes.ok) {
    throw new Error(`manifest の取得に失敗しました (${manifestRes.status})`);
  }
  const manifest: { db: string } = await manifestRes.json();

  const dbRes = await fetch(manifest.db);
  if (!dbRes.ok) {
    throw new Error(`DB の取得に失敗しました (${dbRes.status})`);
  }
  const bytes = new Uint8Array(await dbRes.arrayBuffer());

  const SQL = await initSqlJs({ locateFile: () => wasmUrl });
  return new SQL.Database(bytes);
}

function selectAll(db: Database, sql: string): SqlValue[][] {
  const result = db.exec(sql);
  return result.length === 0 ? [] : result[0].values;
}

function selectOne(db: Database, sql: string, params: SqlValue[]): SqlValue[] | null {
  const stmt = db.prepare(sql);
  try {
    stmt.bind(params);
    return stmt.step() ? stmt.get() : null;
  } finally {
    stmt.free();
  }
}

export function listPosts(db: Database): PostSummary[] {
  return selectAll(
    db,
    "SELECT slug, title, published_at FROM posts ORDER BY published_at DESC, slug DESC",
  ).map(([slug, title, publishedAt]) => ({
    slug: slug as string,
    title: title as string,
    publishedAt: publishedAt as string,
  }));
}

export function getPost(db: Database, slug: string): Post | null {
  const row = selectOne(db, "SELECT slug, title, published_at, body_md FROM posts WHERE slug = ?", [slug]);
  if (!row) {
    return null;
  }
  const [s, title, publishedAt, bodyMd] = row;
  return {
    slug: s as string,
    title: title as string,
    publishedAt: publishedAt as string,
    bodyMd: bodyMd as string,
  };
}

export function listArticles(db: Database): ArticleSummary[] {
  return selectAll(
    db,
    "SELECT slug, title, published_at, description FROM articles ORDER BY published_at DESC, slug DESC",
  ).map(([slug, title, publishedAt, description]) => ({
    slug: slug as string,
    title: title as string,
    publishedAt: publishedAt as string,
    description: (description as string | null) ?? null,
  }));
}

export function getArticle(db: Database, slug: string): Article | null {
  const row = selectOne(
    db,
    "SELECT slug, title, published_at, updated_at, description, body_md FROM articles WHERE slug = ?",
    [slug],
  );
  if (!row) {
    return null;
  }
  const [s, title, publishedAt, updatedAt, description, bodyMd] = row;
  return {
    slug: s as string,
    title: title as string,
    publishedAt: publishedAt as string,
    updatedAt: (updatedAt as string | null) ?? null,
    description: (description as string | null) ?? null,
    bodyMd: bodyMd as string,
  };
}

export function getPage(db: Database, slug: string): Page | null {
  const row = selectOne(db, "SELECT slug, title, body_md FROM pages WHERE slug = ?", [slug]);
  if (!row) {
    return null;
  }
  const [s, title, bodyMd] = row;
  return { slug: s as string, title: title as string, bodyMd: bodyMd as string };
}
