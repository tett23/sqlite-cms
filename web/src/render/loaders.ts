import { createLoader } from "../lazyLoader";
import type { MathRequest } from "./katex.worker";

/** 数式を組む。Worker で組むときは Promise を返す。書き誤りがあれば null。 */
export interface MathRenderer {
  renderMath(tex: string, displayMode: boolean): string | null | Promise<string | null>;
}

/**
 * KaTeX のよく使うフォント。KaTeX の CSS はフォントを読み終わるまで文字を描かない（font-display: block）ので、
 * 組んだ後にフォントが届くと、数式の大きさが変わって本文がずれる。読み込みのときに一緒に読み、届いてから組む（ADR 0051）。
 */
const KATEX_FONTS = ["1em KaTeX_Main", "italic 1em KaTeX_Math"];

/** Worker で数式を組む（ADR 0053）。KaTeX の JS は Worker が読み込む。 */
function workerMath(): MathRenderer {
  const worker = new Worker(new URL("./katex.worker.ts", import.meta.url), { type: "module" });
  const pending = new Map<number, (html: string | null) => void>();
  let nextId = 0;
  worker.onmessage = (event: MessageEvent<{ id: number; html: string | null }>) => {
    pending.get(event.data.id)?.(event.data.html);
    pending.delete(event.data.id);
  };
  // Worker が動かなければ、組めなかった数式として TeX の文字列のまま出す。
  worker.onerror = () => {
    for (const resolve of pending.values()) resolve(null);
    pending.clear();
  };
  return {
    renderMath: (tex, displayMode) =>
      new Promise((resolve) => {
        const id = ++nextId;
        pending.set(id, resolve);
        worker.postMessage({ id, tex, displayMode } satisfies MathRequest);
      }),
  };
}

// テスト（Node.js）には Worker がないので、同じスレッドで同期的に組む（ビルドではこの分岐は消える）。
export const katexLoader = createLoader<MathRenderer>(async () => {
  if (import.meta.env.MODE === "test") return import("./katex");
  await import("katex/dist/katex.min.css");
  // フォントを読めなくても、数式は組む（代わりのフォントで描かれる）。
  await Promise.all(KATEX_FONTS.map((font) => document.fonts.load(font))).catch(() => {});
  return workerMath();
});
export const mermaidLoader = createLoader(() => import("./mermaid"));
// リンクカードの部品（ADR 0049）。URL だけの行が画面の近くに来てから読み込む。
export const linkCardLoader = createLoader(() => import("./LinkCard"));
