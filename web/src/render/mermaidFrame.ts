// 図を描くための作業用のページ（mermaid-frame.html）で動く（ADR 0055）。
// 本体のページの sandbox の iframe の中で mermaid を動かし、描いた SVG を本体に返す。
// オリジンを持たない iframe は本体と別のプロセスで動くので、mermaid の読み込みと図の配置が本体の入力への応答を止めない。
// 本体の「使っていない JS」にも数えられない（mermaid は大きく、図の種類ごとに使わない部分が多い）。

// mermaid は、図の種類ごとの部品を、必要になったときに読み込む（Vite が、読み込む前に modulepreload のリンクを head に足す）。
// このページの中の読み込みは、低い優先度で取る。本体のページの描画には要らないので、本体の読み込みと帯域を取り合わないようにする。
// また、Lighthouse は、別のプロセスの iframe の中でスクリプトが読んだものを本体の HTML からの重要な読み込みとみなし、
// 「ネットワークの依存関係ツリー」の連鎖に数える。優先度の低い読み込みは数えない。
const append = HTMLHeadElement.prototype.appendChild;
HTMLHeadElement.prototype.appendChild = function <T extends Node>(this: HTMLHeadElement, node: T): T {
  if (node instanceof HTMLLinkElement && node.rel === "modulepreload") node.fetchPriority = "low";
  return append.call(this, node) as T;
};

export type FrameRequest = { id: number; source: string };
export type FrameReply = { type: "ready" } | { id: number; svg: string } | { id: number; error: string };

let renderer: Promise<typeof import("./mermaid")> | null = null;

addEventListener("message", (event: MessageEvent<FrameRequest>) => {
  // 本体のページ（この iframe を置いた親）からの頼みだけを受ける。
  if (event.source !== parent) return;
  const { id, source } = event.data;
  renderer ??= import("./mermaid");
  renderer
    .then((mermaid) => mermaid.renderDiagram(source))
    .then(
      (svg) => parent.postMessage({ id, svg } satisfies FrameReply, "*"),
      (error: unknown) => parent.postMessage({ id, error: error instanceof Error ? error.message : String(error) } satisfies FrameReply, "*"),
    );
});

// mermaid を読み込む前に、頼みを受けられることを知らせる（読み込みは最初の頼みで始める）。
parent.postMessage({ type: "ready" } satisfies FrameReply, "*");
