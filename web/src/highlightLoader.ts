import type { HighlightedBlock } from "./highlight";
import { createLoader } from "./lazyLoader";

export interface Highlight {
  highlightBlock(code: string, languageClass: string): HighlightedBlock | null;
}

// Shiki は大きい（gzip 後で約 160 KB）ので、本体とは別のチャンクにして非同期で読み込む。
// 読み込んだときに、色分けの準備（言語とテーマの登録）まで済ませる。最初のブロックの色分けと同じタスクで準備すると、長いタスクになる（ADR 0038）。
export const highlightLoader = createLoader<Highlight>(() =>
  import("./highlight").then((module) => {
    module.getHighlighter();
    return { highlightBlock: module.highlightBlock };
  }),
);
