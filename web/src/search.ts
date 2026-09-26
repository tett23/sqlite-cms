// 全文検索（ADR 0031）。
// sql.js の既定のビルドには FTS5 がないので、FTS4 を使う。FTS4 の分かち書き（simple）は日本語を区切れないので、
// 文字列をこちらで語に分けてから索引に入れる。英数字などは語のまま、日本語は 2 文字ずつ重ねて区切る（bigram）。
// 索引は DB に入れず、最初に検索したときにブラウザで作る（DB を大きくしないため。記事の数が少ないので速い）。
import type { Database, SqlValue } from "sql.js";

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

function bigrams(run: string): string[] {
  const chars = [...run];
  if (chars.length < 2) return chars;
  return chars.slice(0, -1).map((char, i) => char + chars[i + 1]);
}

/** 索引に入れる語の並び。日本語は 2 文字ずつ重ねて区切る。 */
export function tokenize(text: string): string[] {
  return runs(text).flatMap((run) => (run.cjk ? bigrams(run.text) : [run.text]));
}

/** 検索する言葉を、空白で区切った語に分ける。 */
export function queryTerms(query: string): string[] {
  return normalize(query)
    .split(/\s+/)
    .filter((term) => runs(term).length > 0);
}

/**
 * 語を FTS4 の検索式にする。語の中の並びは、隣り合う語の句（"…"）にする。
 * 最後が英数字の語なら前方一致にする。1 文字の日本語だけの語は索引にない（2 文字ずつ区切るため）ので、式にしない。
 */
export function toMatchExpression(terms: string[]): string | null {
  const phrases = terms.flatMap((term) => {
    const termRuns = runs(term);
    if (termRuns.length === 1 && termRuns[0].cjk && [...termRuns[0].text].length < 2) return [];
    const tokens = termRuns.flatMap((run) => (run.cjk ? bigrams(run.text) : [run.text]));
    const last = termRuns[termRuns.length - 1];
    const phrase = `"${tokens.join(" ")}${last.cjk ? "" : "*"}"`;
    return [phrase];
  });
  return phrases.length > 0 ? phrases.join(" ") : null;
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

function selectAll(db: Database, sql: string, params: SqlValue[] = []): SqlValue[][] {
  const stmt = db.prepare(sql);
  try {
    stmt.bind(params);
    const rows: SqlValue[][] = [];
    while (stmt.step()) rows.push(stmt.get());
    return rows;
  } finally {
    stmt.free();
  }
}

const indexed = new WeakSet<Database>();

/** 検索の索引を作る（同じ DB では一度だけ）。一時的な表なので、DB のファイルには残らない。 */
/** 検索のページのパス（サイトを置くパスは含まない）。言葉は前後の空白を除いて `?q=` に入れ、空なら付けない（ADR 0041）。 */
export function searchPath(query: string): string {
  const q = query.trim();
  return q ? `/search?q=${encodeURIComponent(q)}` : "/search";
}

export function ensureSearchIndex(db: Database) {
  if (indexed.has(db)) return;
  const documents: Document[] = [
    ...selectAll(db, "SELECT slug, title, published_at, body_md FROM posts").map(([slug, title, date, body]) => ({
      kind: "post" as const,
      slug: slug as string,
      title: title as string,
      date: date as string,
      text: plainText(body as string),
    })),
    ...selectAll(db, "SELECT slug, title, published_at, description, body_md FROM articles").map(
      ([slug, title, date, description, body]) => ({
        kind: "article" as const,
        slug: slug as string,
        title: title as string,
        date: date as string,
        text: plainText(`${(description as string | null) ?? ""}\n${body as string}`),
      }),
    ),
    // ページは自己紹介だけに URL がある。
    ...selectAll(db, "SELECT slug, title, body_md FROM pages WHERE slug = 'about'").map(([slug, title, body]) => ({
      kind: "page" as const,
      slug: slug as string,
      title: title as string,
      date: null,
      text: plainText(body as string),
    })),
  ];
  db.run("DROP TABLE IF EXISTS temp.search_documents");
  db.run("DROP TABLE IF EXISTS temp.search_index");
  db.run("CREATE TEMP TABLE search_documents (id INTEGER PRIMARY KEY, kind TEXT, slug TEXT, title TEXT, date TEXT, text TEXT)");
  db.run("CREATE VIRTUAL TABLE temp.search_index USING fts4(title, body, tokenize=simple)");
  documents.forEach((doc, i) => {
    db.run("INSERT INTO temp.search_documents VALUES (?, ?, ?, ?, ?, ?)", [i + 1, doc.kind, doc.slug, doc.title, doc.date, doc.text]);
    db.run("INSERT INTO temp.search_index (rowid, title, body) VALUES (?, ?, ?)", [
      i + 1,
      tokenize(doc.title).join(" "),
      tokenize(doc.text).join(" "),
    ]);
  });
  indexed.add(db);
}

/**
 * 検索する。すべての語を含むものを、題名に語を含むものを先に、新しい順に返す。
 * 索引で候補を絞ってから、元の文字列に語が含まれることを確かめる（1 文字の日本語の語も、ここで確かめる）。
 */
export function search(db: Database, query: string): SearchResult[] {
  const terms = queryTerms(query);
  if (terms.length === 0) return [];
  ensureSearchIndex(db);
  const expression = toMatchExpression(terms);
  const rows = expression
    ? selectAll(
        db,
        "SELECT d.kind, d.slug, d.title, d.date, d.text FROM search_index JOIN search_documents AS d ON d.id = search_index.rowid WHERE search_index MATCH ?",
        [expression],
      )
    : selectAll(db, "SELECT kind, slug, title, date, text FROM temp.search_documents");
  const results = rows
    .map(([kind, slug, title, date, text]) => ({
      kind: kind as SearchKind,
      slug: slug as string,
      title: title as string,
      date: date as string | null,
      text: text as string,
    }))
    .filter((doc) => {
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
