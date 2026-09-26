// KaTeX は大きい（JS が gzip 後で約 75 KB、ほかにフォント）ので、数式のあるページでだけ読み込む（ADR 0025）。
import katex from "katex";
import "katex/dist/katex.min.css";

/** 数式を HTML にする。読み上げ用に MathML も出力する。書き誤りがあれば null。 */
export function renderMath(tex: string, displayMode: boolean): string | null {
  try {
    return katex.renderToString(tex, { displayMode, throwOnError: true, output: "htmlAndMathml" });
  } catch {
    return null;
  }
}
