import { createLoader } from "../lazyLoader";

export const katexLoader = createLoader(() => import("./katex"));
export const mermaidLoader = createLoader(() => import("./mermaid"));
// リンクカードの部品（ADR 0049）。URL だけの行が画面の近くに来てから読み込む。
export const linkCardLoader = createLoader(() => import("./LinkCard"));
