// 全文検索（ADR 0031、0047）。
// すべての文書（post、article、自己紹介）の題名と本文を正規化し、検索する言葉のすべての語を含むものを探す。
// 以前は sql.js の FTS4 の索引で候補を絞っていたが、最後にこの確かめで結果を決めていたので、索引なしでも結果は同じになる。
// 記事の数が少ないので、すべての文書を確かめても速い。文書は最初に検索したときに一度だけ作る。
import type { SqliteFile } from "./sqlite";

export type SearchKind = "post" | "article" | "page";

export interface SnippetPart {
  text: string;
  match: boolean;
}

export interface SearchResult {
  kind: SearchKind;
  slug: string;
  title: string;
  date: string | null;
  /** サイトの中のパス。 */
  path: string;
  /** 本文の、最初に語が現れるあたりの抜き出し。 */
  snippet: SnippetPart[];
}

// ひらがな、カタカナ、漢字、半角カタカナ、長音と繰り返しの記号。2 文字ずつ区切る。
const CJK = /[぀-ヿ㐀-䶿一-鿿豈-﫿ｦ-ﾟ々〆ー]/u;
const WORD = /[\p{L}\p{N}]/u;

/** 検索で比べるための正規化。全角の英数字を半角にし（NFKC）、小文字にする。 */
export function normalize(text: string): string {
  return text.normalize("NFKC").toLowerCase();
}

/** 文字列を、英数字などの語の並びと、日本語の並びに分ける。 */
function runs(text: string): { cjk: boolean; text: string }[] {
  const out: { cjk: boolean; text: string }[] = [];
  let current: { cjk: boolean; text: string } | null = null;
  for (const char of normalize(text)) {
    const cjk = CJK.test(char);
    if (!cjk && !WORD.test(char)) {
      current = null;
      continue;
    }
    if (current && current.cjk === cjk) {
      current.text += char;
    } else {
      current = { cjk, text: char };
      out.push(current);
    }
  }
  return out;
}

/** 検索する言葉を、空白で区切った語に分ける。記号だけの語は除く。 */
export function queryTerms(query: string): string[] {
  return normalize(query)
    .split(/\s+/)
    .filter((term) => runs(term).length > 0);
}

/** 本文の Markdown から、検索と抜き出しに使う文字列を作る（記号を大まかに取り除く）。 */
export function plainText(markdown: string): string {
  return markdown
    .replace(/<!--[\s\S]*?-->/g, " ")
    .replace(/<[^>]+>/g, " ")
    .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/^\s{0,3}(#{1,6}|>|[-*+]|\d+\.)\s+/gm, "")
    .replace(/^\s*(:{3,}|`{3,}|~{3,}|\${2}).*$/gm, " ")
    .replace(/[*_~`]/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

const SNIPPET_BEFORE = 30;
const SNIPPET_LENGTH = 110;

/** 最初に語が現れるあたりを抜き出し、語の部分に印を付ける。 */
export function snippet(text: string, terms: string[]): SnippetPart[] {
  const lower = normalize(text);
  // NFKC で長さが変わる文字があると位置がずれるので、変わらないときだけ位置を使う。
  const comparable = lower.length === text.length;
  const hits = comparable ? terms.map((term) => lower.indexOf(term)).filter((i) => i >= 0) : [];
  const first = hits.length > 0 ? Math.min(...hits) : 0;
  const start = Math.max(0, first - SNIPPET_BEFORE);
  const end = Math.min(text.length, start + SNIPPET_LENGTH);
  const parts: SnippetPart[] = [];
  let position = start;
  while (position < end && comparable) {
    let next = -1;
    let length = 0;
    for (const term of terms) {
      const index = lower.indexOf(term, position);
      if (index >= 0 && index < end && (next < 0 || index < next)) {
        next = index;
        length = term.length;
      }
    }
    if (next < 0) break;
    if (next > position) parts.push({ text: text.slice(position, next), match: false });
    parts.push({ text: text.slice(next, Math.min(end, next + length)), match: true });
    position = next + length;
  }
  if (position < end) parts.push({ text: text.slice(position, end), match: false });
  if (start > 0) parts.unshift({ text: "…", match: false });
  if (end < text.length) parts.push({ text: "…", match: false });
  return parts;
}

interface Document {
  kind: SearchKind;
  slug: string;
  title: string;
  date: string | null;
  text: string;
}

const PATHS: Record<SearchKind, (slug: string) => string> = {
  post: (slug) => `/posts/${slug}`,
  article: (slug) => `/articles/${slug}`,
  page: () => "/about",
};

const documentCache = new WeakMap<SqliteFile, Document[]>();

/** 検索のページのパス（サイトを置くパスは含まない）。言葉は前後の空白を除いて `?q=` に入れ、空なら付けない（ADR 0041）。 */
export function searchPath(query: string): string {
  const q = query.trim();
  return q ? `/search?q=${encodeURIComponent(q)}` : "/search";
}

/** 検索する文書（同じ DB では一度だけ作る）。 */
function documents(db: SqliteFile): Document[] {
  let docs = documentCache.get(db);
  if (!docs) {
    const text = (value: unknown) => (typeof value === "string" ? value : null);
    docs = [
      ...db.table("posts").map((row) => ({
        kind: "post" as const,
        slug: text(row.slug)!,
        title: text(row.title)!,
        date: text(row.published_at),
        text: plainText(text(row.body_md)!),
      })),
      ...db.table("articles").map((row) => ({
        kind: "article" as const,
        slug: text(row.slug)!,
        title: text(row.title)!,
        date: text(row.published_at),
        text: plainText(`${text(row.description) ?? ""}\n${text(row.body_md)!}`),
      })),
      // ページは自己紹介だけに URL がある。
      ...db
        .table("pages")
        .filter((row) => row.slug === "about")
        .map((row) => ({ kind: "page" as const, slug: text(row.slug)!, title: text(row.title)!, date: null, text: plainText(text(row.body_md)!) })),
    ];
    documentCache.set(db, docs);
  }
  return docs;
}

/**
 * 検索する。すべての語を含むものを、題名に語を含むものを先に、新しい順に返す。
 */
export function search(db: SqliteFile, query: string): SearchResult[] {
  const terms = queryTerms(query);
  if (terms.length === 0) return [];
  const results = documents(db).filter((doc) => {
    const haystack = normalize(`${doc.title}\n${doc.text}`);
    return terms.every((term) => haystack.includes(term));
  });
  const titleHits = (doc: { title: string }) => terms.filter((term) => normalize(doc.title).includes(term)).length;
  results.sort((a, b) => titleHits(b) - titleHits(a) || (b.date ?? "").localeCompare(a.date ?? ""));
  return results.map((doc) => ({
    kind: doc.kind,
    slug: doc.slug,
    title: doc.title,
    date: doc.date,
    path: PATHS[doc.kind](doc.slug),
    snippet: snippet(doc.text, terms),
  }));
}
