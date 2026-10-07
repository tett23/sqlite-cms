import { createLoader } from "../lazyLoader";
import { withBasePath } from "../base";
import type { MathRequest } from "./katex.worker";
import type { FrameReply, FrameRequest } from "./mermaidFrame";

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
/** 図を SVG にする。書き誤りがあれば reject する。 */
export interface DiagramRenderer {
  renderDiagram(source: string): Promise<string>;
}

/** 作業用のページが、頼みを受けられることを知らせるまで待つ時間。過ぎたら、本体のページで描く。 */
const FRAME_READY_TIMEOUT = 10_000;
/**
 * 作業用のページを読み終えた（load）後に、知らせを待つ時間。
 * 知らせは load より前に送られる。iframe を止められた（広告を止める拡張など）ときや、
 * CORS のヘッダがなくて JS を読めなかったときも load は来るので、そこから短く待って、本体のページで描く。
 */
const FRAME_AFTER_LOAD_TIMEOUT = 1_000;

/**
 * 図を、sandbox の iframe（mermaid-frame.html）の中の mermaid で描く（ADR 0055）。
 * オリジンを持たない iframe から JS を読むので、配信先が /assets/ に Access-Control-Allow-Origin を付けている必要がある。
 * 付いていない（作業用のページが知らせてこない）ときは reject し、呼び出し側が本体のページで描く。
 */
function frameMermaid(): Promise<DiagramRenderer> {
  return new Promise((resolve, reject) => {
    const frame = document.createElement("iframe");
    // allow-same-origin を付けない。オリジンを持たない iframe は、本体と別のプロセスで動く。
    frame.setAttribute("sandbox", "allow-scripts");
    frame.setAttribute("aria-hidden", "true");
    frame.tabIndex = -1;
    frame.title = "図を描くための作業用の枠";
    // 見えないが、文字の大きさを測れるよう、大きさを持たせて画面の外に置く（display: none では測れない）。
    frame.style.cssText = "position:fixed;left:-10000px;top:0;width:1024px;height:768px;border:0;visibility:hidden";
    frame.src = withBasePath("/mermaid-frame.html");
    const pending = new Map<number, { resolve(svg: string): void; reject(error: Error): void }>();
    let nextId = 0;
    let ready = false;
    const fail = () => {
      if (ready) return;
      clearTimeout(timer);
      removeEventListener("message", onMessage);
      frame.remove();
      reject(new Error("図を描くための作業用のページが応えません"));
    };
    const timer = setTimeout(fail, FRAME_READY_TIMEOUT);
    frame.addEventListener("load", () => setTimeout(fail, FRAME_AFTER_LOAD_TIMEOUT), { once: true });
    function onMessage(event: MessageEvent<FrameReply>) {
      if (event.source !== frame.contentWindow) return;
      const reply = event.data;
      if ("type" in reply) {
        if (ready) return;
        ready = true;
        clearTimeout(timer);
        resolve({
          renderDiagram: (source) =>
            new Promise((resolveSvg, rejectSvg) => {
              const id = ++nextId;
              pending.set(id, { resolve: resolveSvg, reject: rejectSvg });
              frame.contentWindow?.postMessage({ id, source } satisfies FrameRequest, "*");
            }),
        });
        return;
      }
      const request = pending.get(reply.id);
      pending.delete(reply.id);
      if ("svg" in reply) request?.resolve(reply.svg);
      else request?.reject(new Error(reply.error));
    }
    addEventListener("message", onMessage);
    document.body.append(frame);
  });
}

// テスト（Node.js）には iframe がないので、同じスレッドで描く（ビルドではこの分岐は消える）。
export const mermaidLoader = createLoader<DiagramRenderer>(async () => {
  if (import.meta.env.MODE === "test") return import("./mermaid");
  return frameMermaid().catch(() => import("./mermaid"));
});
// リンクカードの部品（ADR 0049）。URL だけの行が画面の近くに来てから読み込む。
export const linkCardLoader = createLoader(() => import("./LinkCard"));
