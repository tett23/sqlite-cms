import type { HighlightedBlock } from "./highlight";
import type { HighlightRequest } from "./highlight.worker";
import { createLoader } from "./lazyLoader";

export interface Highlight {
  /** コードブロックを色分けする。Worker で色分けするときは Promise を返す。 */
  highlightBlock(code: string, languageClass: string): HighlightedBlock | null | Promise<HighlightedBlock | null>;
}

type Reply = { id: number; block?: HighlightedBlock | null; error?: string };

/** Worker で色分けする（ADR 0046）。Shiki と正規表現エンジンの wasm は、Worker が読み込む。 */
function workerHighlight(): Highlight {
  const worker = new Worker(new URL("./highlight.worker.ts", import.meta.url), { type: "module" });
  const pending = new Map<number, { resolve(block: HighlightedBlock | null): void; reject(error: Error): void }>();
  let nextId = 0;
  worker.onmessage = (event: MessageEvent<Reply>) => {
    const { id, block, error } = event.data;
    const request = pending.get(id);
    pending.delete(id);
    if (error !== undefined) request?.reject(new Error(error));
    else request?.resolve(block ?? null);
  };
  worker.onerror = (event) => {
    for (const request of pending.values()) request.reject(new Error(event.message));
    pending.clear();
  };
  return {
    highlightBlock: (code, languageClass) =>
      new Promise((resolve, reject) => {
        const id = ++nextId;
        pending.set(id, { resolve, reject });
        worker.postMessage({ id, code, languageClass } satisfies HighlightRequest);
      }),
  };
}

// Shiki は大きい（gzip 後で約 150 KB と、正規表現エンジンの wasm が約 160 KB）ので、必要になったときに初めて読み込む（ADR 0029）。
// ブラウザでは Worker で色分けする。テスト（Node.js）には Worker がないので、同じスレッドで同期的に色分けする（ビルドではこの分岐は消える）。
export const highlightLoader = createLoader<Highlight>(async () => {
  if (import.meta.env.MODE === "test") {
    const module = await import("./highlight");
    await module.initHighlighter();
    return { highlightBlock: module.highlightBlock };
  }
  return workerHighlight();
});
