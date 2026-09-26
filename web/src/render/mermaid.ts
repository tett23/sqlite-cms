// mermaid は非常に大きい（gzip 後で数百 KB）ので、図のあるページでだけ読み込む（ADR 0025）。
import mermaid from "mermaid";

mermaid.initialize({
  startOnLoad: false,
  // 図の中のクリックの処理や HTML を許さない（Zenn と同じ）。
  securityLevel: "strict",
  theme: "neutral",
  fontFamily: "sans-serif",
  logLevel: "fatal",
  // mermaid 12 は既定のレイアウトが elk で、フローチャート、状態遷移図、ER 図などを描くたびに elk（gzip 後で約 440 KB）を読み込む。
  // 既定は dagre にし、elk は図で `layout: elk` を指定したときだけ使う（ADR 0025）。
  layout: "dagre",
});

let count = 0;

/** 図を SVG にする。書き誤りがあれば例外を投げる。 */
export async function renderDiagram(source: string): Promise<string> {
  // 先に構文を確かめる。誤りのある図を render に渡すと、誤りを表す図を描いてしまう。
  await mermaid.parse(source);
  const { svg } = await mermaid.render(`mermaid-${++count}`, source);
  return svg;
}
