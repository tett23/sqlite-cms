// KaTeX は大きい（JS が gzip 後で約 75 KB、ほかにフォント）ので、数式のあるページでだけ読み込む（ADR 0025）。
// ブラウザでは Worker（katex.worker.ts）の中で組む（ADR 0053）。CSS とフォントは、読み込むときにメインスレッドで読む（loaders.ts）。
import katex from "katex";

/** 数式を HTML にする。読み上げ用に MathML も出力する。書き誤りがあれば null。 */
export function renderMath(tex: string, displayMode: boolean): string | null {
  try {
    return katex.renderToString(tex, { displayMode, throwOnError: true, output: "htmlAndMathml" });
  } catch {
    return null;
  }
}
