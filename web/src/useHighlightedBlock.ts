import { useEffect, useState } from "react";
import type { HighlightedBlock } from "./highlight";
import { highlightLoader } from "./highlightLoader";
import { inTurn, useLazy } from "./lazyLoader";
import { useNearViewport } from "./nearViewport";

const logHighlightError = (error: unknown) => console.error("シンタックスハイライトを読み込めませんでした", error);

/**
 * コードブロックを色付けする。返す ref を pre に付ける。色付けできるまでは null を返す。
 * 色を付けられる言語のコードブロックが画面の近くに来たときに、初めて Shiki を読み込み、そのブロックだけを色付けする（ADR 0029、0038）。
 * 本文の Markdown 全体は描き直さない。highlightable が false なら Shiki を読み込まない。
 */
export function useHighlightedBlock(
  source: string,
  languageClass: string | undefined,
  highlightable: boolean,
): [(element: HTMLPreElement | null) => void, HighlightedBlock | null] {
  const [ref, near] = useNearViewport<HTMLPreElement>();
  const highlight = useLazy(highlightLoader, logHighlightError, near && highlightable);
  // 色分けは、ブロックごとに別のタスクで行う（inTurn）。読み込み済みで、初めから画面の近くにあるとき（サーバーでの描画）だけ、最初の描画で行う。
  // 同じ部品が別の記事のコードブロックに使い回されることがあるので、色分けの結果は、言語とコードの組と一緒に持つ。
  const key = `${languageClass}\n${source}`;
  const canHighlight = highlight !== null && near && highlightable && languageClass !== undefined;
  const [highlighted, setHighlighted] = useState<{ key: string; block: HighlightedBlock | null } | null>(() => {
    if (!canHighlight) return null;
    const block = highlight.highlightBlock(source, languageClass);
    return block instanceof Promise ? null : { key, block };
  });
  const done = highlighted?.key === key;
  useEffect(() => {
    if (done || !canHighlight) return;
    let cancelled = false;
    // 色分けの結果（Worker なら届いた結果）を、ブロックごとに別のタスクで描く（ADR 0038、0046）。
    const cancelTurn = inTurn(() => {
      void Promise.resolve(highlight.highlightBlock(source, languageClass)).then(
        (block) => !cancelled && setHighlighted({ key, block }),
        logHighlightError,
      );
    });
    return () => {
      cancelled = true;
      cancelTurn();
    };
  }, [done, canHighlight, highlight, key, source, languageClass]);
  return [ref, done ? highlighted.block : null];
}
