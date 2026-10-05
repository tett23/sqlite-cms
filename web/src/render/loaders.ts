import { createLoader } from "../lazyLoader";

/**
 * KaTeX のよく使うフォント。KaTeX の CSS はフォントを読み終わるまで文字を描かない（font-display: block）ので、
 * 組んだ後にフォントが届くと、数式の大きさが変わって本文がずれる。読み込みのときに一緒に読み、届いてから組む（ADR 0051）。
 */
const KATEX_FONTS = ["1em KaTeX_Main", "italic 1em KaTeX_Math"];

export const katexLoader = createLoader(async () => {
  const module = await import("./katex");
  // フォントを読めなくても、数式は組む（代わりのフォントで描かれる）。
  await Promise.all(KATEX_FONTS.map((font) => globalThis.document?.fonts?.load(font))).catch(() => {});
  return module;
});
export const mermaidLoader = createLoader(() => import("./mermaid"));
// リンクカードの部品（ADR 0049）。URL だけの行が画面の近くに来てから読み込む。
export const linkCardLoader = createLoader(() => import("./LinkCard"));
