import type { Database } from "sql.js";
import { search, type SearchResult } from "./search";

/** ヘッダの検索ボックスに出す候補の数（ADR 0042）。 */
export const SUGGESTION_LIMIT = 5;

/** 入力中の言葉の候補。検索のページと同じ順（題名に言葉を含むものが先、新しい順）で、先頭の SUGGESTION_LIMIT 件。 */
export function suggest(db: Database, query: string): SearchResult[] {
  return query.trim() ? search(db, query).slice(0, SUGGESTION_LIMIT) : [];
}

/**
 * 矢印のキーで選ぶ候補を動かす。-1 は、どの候補も選んでいない（入力欄にいる）ことを表す。
 * 最後の候補の次と最初の候補の前は、入力欄に戻る。
 */
export function moveActive(active: number, delta: 1 | -1, count: number): number {
  if (count === 0) return -1;
  return ((active + 1 + delta + count + 1) % (count + 1)) - 1;
}
