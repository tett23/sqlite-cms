// 全文検索（ADR 0031、0047）。
// すべての文書（post、article、自己紹介）の題名と本文を正規化し、検索する言葉のすべての語を含むものを探す。
// 以前は sql.js の FTS4 の索引で候補を絞っていたが、最後にこの確かめで結果を決めていたので、索引なしでも結果は同じになる。
// 記事の数が少ないので、すべての文書を確かめても速い。文書は最初に検索したときに一度だけ作る。
import { tagsOf } from "./db";
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
  /** タグ（ADR 0048）。先頭の # は付かない。 */
  tags: string[];
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
  tags: string[];
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
        tags: tagsOf(db, "post", text(row.slug)!),
      })),
      ...db.table("articles").map((row) => ({
        kind: "article" as const,
        slug: text(row.slug)!,
        title: text(row.title)!,
        date: text(row.published_at),
        text: plainText(`${text(row.description) ?? ""}\n${text(row.body_md)!}`),
        tags: tagsOf(db, "article", text(row.slug)!),
      })),
      // ページは自己紹介だけに URL がある。
      ...db
        .table("pages")
        .filter((row) => row.slug === "about")
        .map((row) => ({
          kind: "page" as const,
          slug: text(row.slug)!,
          title: text(row.title)!,
          date: null,
          text: plainText(text(row.body_md)!),
          tags: [],
        })),
    ];
    documentCache.set(db, docs);
  }
  return docs;
}

/** 検索する言葉を、語（本文と題名とタグから探す）と、タグ（`#` で始まる語。そのタグの付いた記事だけに一致する）に分ける（ADR 0048）。 */
export function parseQuery(query: string): { terms: string[]; tags: string[] } {
  const terms: string[] = [];
  const tags: string[] = [];
  // 全角の ＃ は、正規化（NFKC）で # になる。
  for (const word of normalize(query).split(/\s+/)) {
    if (word.startsWith("#")) {
      const tag = word.replace(/^#+/, "");
      if (tag) tags.push(tag);
    } else if (runs(word).length > 0) {
      terms.push(word);
    }
  }
  return { terms, tags };
}

/** タグで探すための言葉（`#タグ`）。タグのリンクと、検索の欄に入れる言葉に使う（ADR 0048）。 */
export function tagQuery(tag: string): string {
  return `#${tag}`;
}

/**
 * 検索する。すべての語を含み、すべてのタグの付いたものを、題名に語を含むものを先に、新しい順に返す。
 * 語は、題名、タグ、本文から探す。`#` で始まる語は、タグがちょうど一致するものだけを探す（大文字と小文字、全角と半角は区別しない）。
 */
export function search(db: SqliteFile, query: string): SearchResult[] {
  const { terms, tags } = parseQuery(query);
  if (terms.length === 0 && tags.length === 0) return [];
  const results = documents(db).filter((doc) => {
    const docTags = doc.tags.map(normalize);
    if (!tags.every((tag) => docTags.includes(tag))) return false;
    const haystack = normalize(`${doc.title}\n${doc.tags.join(" ")}\n${doc.text}`);
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
    tags: doc.tags,
    snippet: snippet(doc.text, terms),
  }));
}
