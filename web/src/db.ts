import { withBasePath } from "./base";
import { compareBinary, SqliteFile, type Row } from "./sqlite";

export interface PostSummary {
  slug: string;
  title: string;
  publishedAt: string;
}

export interface Post extends PostSummary {
  bodyMd: string;
  /** タグ（ADR 0048）。先頭の # は付かない。 */
  tags: string[];
}

export interface ArticleSummary extends PostSummary {
  description: string | null;
}

export interface Article extends ArticleSummary {
  updatedAt: string | null;
  bodyMd: string;
  /** タグ（ADR 0048）。先頭の # は付かない。 */
  tags: string[];
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
 * 自前の読み手（SqliteFile）で読み、sql.js（wasm）は使わない（ADR 0047）。
 */
export async function loadDb(): Promise<SqliteFile> {
  const embedded = embeddedDbPath();
  let dbRes = embedded ? await fetch(embedded).catch(() => null) : null;
  if (!dbRes?.ok) {
    const path = await manifestDbPath();
    dbRes = await fetch(path);
  }
  if (!dbRes.ok) {
    throw new Error(`DB の取得に失敗しました (${dbRes.status})`);
  }
  return new SqliteFile(new Uint8Array(await dbRes.arrayBuffer()));
}

/** 列の値を文字列として読む（NULL なら null）。 */
const text = (row: Row, column: string) => (row[column] ?? null) as string | null;

/** 新しい順（日付の降順、同じ日付なら slug の降順）。 */
const newestFirst = (a: Row, b: Row) =>
  compareBinary(text(b, "published_at")!, text(a, "published_at")!) || compareBinary(text(b, "slug")!, text(a, "slug")!);

const tagCache = new WeakMap<SqliteFile, Map<string, string[]>>();

/**
 * 記事（kind は post か article）のタグを、書いた順に返す（ADR 0048）。同じ DB では一度だけ読む。
 * タグの表のない DB（タグを入れる前の sqlite-cms で作ったもの）では、空を返す。
 */
export function tagsOf(db: SqliteFile, kind: "post" | "article", slug: string): string[] {
  let tags = tagCache.get(db);
  if (!tags) {
    tags = new Map();
    const rows = db.hasTable("tags") ? [...db.table("tags")] : [];
    rows.sort((a, b) => (a.position as number) - (b.position as number));
    for (const row of rows) {
      const key = `${text(row, "kind")}/${text(row, "slug")}`;
      tags.set(key, [...(tags.get(key) ?? []), text(row, "tag")!]);
    }
    tagCache.set(db, tags);
  }
  return tags.get(`${kind}/${slug}`) ?? [];
}

export function listPosts(db: SqliteFile): PostSummary[] {
  return [...db.table("posts")].sort(newestFirst).map((row) => ({
    slug: text(row, "slug")!,
    title: text(row, "title")!,
    publishedAt: text(row, "published_at")!,
  }));
}

export function getPost(db: SqliteFile, slug: string): Post | null {
  const row = db.table("posts").find((r) => r.slug === slug);
  if (!row) {
    return null;
  }
  return {
    slug: text(row, "slug")!,
    title: text(row, "title")!,
    publishedAt: text(row, "published_at")!,
    bodyMd: text(row, "body_md")!,
    tags: tagsOf(db, "post", slug),
  };
}

export function listArticles(db: SqliteFile): ArticleSummary[] {
  return [...db.table("articles")].sort(newestFirst).map((row) => ({
    slug: text(row, "slug")!,
    title: text(row, "title")!,
    publishedAt: text(row, "published_at")!,
    description: text(row, "description"),
  }));
}

export function getArticle(db: SqliteFile, slug: string): Article | null {
  const row = db.table("articles").find((r) => r.slug === slug);
  if (!row) {
    return null;
  }
  return {
    slug: text(row, "slug")!,
    title: text(row, "title")!,
    publishedAt: text(row, "published_at")!,
    updatedAt: text(row, "updated_at"),
    description: text(row, "description"),
    bodyMd: text(row, "body_md")!,
    tags: tagsOf(db, "article", slug),
  };
}

export function getPage(db: SqliteFile, slug: string): Page | null {
  const row = db.table("pages").find((r) => r.slug === slug);
  if (!row) {
    return null;
  }
  return { slug: text(row, "slug")!, title: text(row, "title")!, bodyMd: text(row, "body_md")! };
}

export function getSite(db: SqliteFile): Site {
  const row = db.table("site").find((r) => r.id === 1);
  if (!row) {
    throw new Error("site テーブルが空です");
  }
  const title = text(row, "title")!;
  const licenseName = text(row, "license_name");
  return {
    title,
    author: text(row, "author"),
    license: licenseName === null ? null : { name: licenseName, url: text(row, "license_url") },
    homeMd: text(row, "home_md"),
    description: text(row, "description") ?? title,
    headerMd: text(row, "header_md"),
  };
}

const linkCardImageCache = new WeakMap<SqliteFile, ReadonlyMap<string, string>>();

/** リンクカードの URL と、その画像のパス（ADR 0028）。同じ DB では一度だけ読む。 */
export function getLinkCardImages(db: SqliteFile): ReadonlyMap<string, string> {
  let images = linkCardImageCache.get(db);
  if (!images) {
    images = new Map(db.table("link_cards").map((row) => [text(row, "url")!, text(row, "image_path")!]));
    linkCardImageCache.set(db, images);
  }
  return images;
}

/** 本文の画像の大きさ（px、ADR 0051）。 */
export interface MediaSize {
  width: number;
  height: number;
}

const mediaSizeCache = new WeakMap<SqliteFile, ReadonlyMap<string, MediaSize>>();

/**
 * 本文の画像（/media/…）のパスと、その大きさ（ADR 0051）。同じ DB では一度だけ読む。
 * 大きさの表のない DB（表を入れる前の sqlite-cms で作ったもの）では、空を返す。
 */
export function getMediaSizes(db: SqliteFile): ReadonlyMap<string, MediaSize> {
  let sizes = mediaSizeCache.get(db);
  if (!sizes) {
    const rows = db.hasTable("media_sizes") ? db.table("media_sizes") : [];
    sizes = new Map(rows.map((row) => [text(row, "path")!, { width: row.width as number, height: row.height as number }]));
    mediaSizeCache.set(db, sizes);
  }
  return sizes;
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
export function listAll(db: SqliteFile): ListEntry[] {
  const entries: ListEntry[] = [
    ...db.table("articles").map((row) => ({
      kind: "article" as const,
      slug: text(row, "slug")!,
      title: text(row, "title")!,
      publishedAt: text(row, "published_at")!,
      description: text(row, "description"),
    })),
    ...db.table("posts").map((row) => ({
      kind: "post" as const,
      slug: text(row, "slug")!,
      title: text(row, "title")!,
      publishedAt: text(row, "published_at")!,
      description: null,
    })),
  ];
  return entries.sort(
    (a, b) => compareBinary(b.publishedAt, a.publishedAt) || compareBinary(a.kind, b.kind) || compareBinary(b.slug, a.slug),
  );
}
