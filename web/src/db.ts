import initSqlJs, { type Database, type SqlValue } from "sql.js";
import wasmUrl from "sql.js/dist/sql-wasm.wasm?url";
import { withBasePath } from "./base";

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

export interface License {
  name: string;
  url: string | null;
}

export interface Site {
  title: string;
  author: string | null;
  license: License | null;
  homeMd: string | null;
  description: string;
  /** ヘッダの Markdown（ADR 0043）。site.toml の値は埋め込み済み。なければ既定のヘッダ。 */
  headerMd: string | null;
}

/** CLI が index.html に入れた DB のパス（ADR 0038）。なければ null。 */
function embeddedDbPath(): string | null {
  if (typeof document === "undefined") return null;
  return document.querySelector('meta[name="sqlite-cms-db"]')?.getAttribute("content") ?? null;
}

/** マニフェストから今の DB のパスを読む。 */
async function manifestDbPath(): Promise<string> {
  const manifestRes = await fetch(withBasePath("/db/manifest.json"), { cache: "no-cache" });
  if (!manifestRes.ok) {
    throw new Error(`manifest の取得に失敗しました (${manifestRes.status})`);
  }
  const manifest: { db: string } = await manifestRes.json();
  return withBasePath(manifest.db);
}

/**
 * DB を読み込む。index.html に DB のパスがあれば、マニフェストを読まずに直接取りに行く（先読みも効く）。
 * 取れなければ（古い index.html が消えた DB を指しているときなど）、マニフェストから読み直す。
 */
export async function loadDb(): Promise<Database> {
  const SQL = initSqlJs({ locateFile: () => wasmUrl });
  const embedded = embeddedDbPath();
  let dbRes = embedded ? await fetch(embedded).catch(() => null) : null;
  if (!dbRes?.ok) {
    const path = await manifestDbPath();
    dbRes = await fetch(path);
  }
  if (!dbRes.ok) {
    throw new Error(`DB の取得に失敗しました (${dbRes.status})`);
  }
  const bytes = new Uint8Array(await dbRes.arrayBuffer());
  return new (await SQL).Database(bytes);
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

export function getSite(db: Database): Site {
  const row = selectOne(
    db,
    "SELECT title, author, license_name, license_url, home_md, description, header_md FROM site WHERE id = 1",
    [],
  );
  if (!row) {
    throw new Error("site テーブルが空です");
  }
  const [title, author, licenseName, licenseUrl, homeMd, description, headerMd] = row;
  return {
    title: title as string,
    author: (author as string | null) ?? null,
    license:
      licenseName === null
        ? null
        : { name: licenseName as string, url: (licenseUrl as string | null) ?? null },
    homeMd: (homeMd as string | null) ?? null,
    description: (description as string | null) ?? (title as string),
    headerMd: (headerMd as string | null) ?? null,
  };
}

const linkCardImageCache = new WeakMap<Database, ReadonlyMap<string, string>>();

/** リンクカードの URL と、その画像のパス（ADR 0028）。同じ DB では一度だけ読む。 */
export function getLinkCardImages(db: Database): ReadonlyMap<string, string> {
  let images = linkCardImageCache.get(db);
  if (!images) {
    images = new Map(
      selectAll(db, "SELECT url, image_path FROM link_cards").map(([url, path]) => [url as string, path as string]),
    );
    linkCardImageCache.set(db, images);
  }
  return images;
}

/** 統合一覧（ADR 0033）の項目。post と article をまとめたもの。 */
export interface ListEntry {
  kind: "post" | "article";
  slug: string;
  title: string;
  publishedAt: string;
  description: string | null;
}

/** post と article を、日付の新しい順（同じ日付なら article、slug の逆順）にまとめて返す。 */
export function listAll(db: Database): ListEntry[] {
  return selectAll(
    db,
    `SELECT 'article', slug, title, published_at, description FROM articles
     UNION ALL
     SELECT 'post', slug, title, published_at, NULL FROM posts
     ORDER BY 4 DESC, 1, 2 DESC`,
  ).map(([kind, slug, title, publishedAt, description]) => ({
    kind: kind as "post" | "article",
    slug: slug as string,
    title: title as string,
    publishedAt: publishedAt as string,
    description: (description as string | null) ?? null,
  }));
}
