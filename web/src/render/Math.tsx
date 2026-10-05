import { useEffect, useState } from "react";
import { inTurn, useLazy } from "../lazyLoader";
import { useNearViewport } from "../nearViewport";
import { MATH_DISPLAY_CLASS, MATH_INLINE_CLASS } from "../markdown/math";
import { katexLoader } from "./loaders";

/** 縦に大きく組む記法（分数、総和、積分、行列など）。 */
const TALL = /\\(?:[dt]?frac|over|sum|prod|int|oint|lim|binom|sqrt|xrightarrow)(?![A-Za-z])|\{[pbvBV]?matrix\}/;
/** 行を \\ で分ける環境。 */
const MULTI_ROW = /\\begin\{(?:aligned|align\*?|gather\*?|gathered|cases|split|array|eqnarray\*?)\}/;

/**
 * ブロックの数式を組んだときの高さの見積もり（rem、ADR 0051）。組む前の場所に、この高さを取っておき、組んだときに本文がずれないようにする。
 * 上下の余白（1em ずつ）と、行ごとの高さ（分数などがあれば 2.9rem、なければ 1.75rem）の和。見本の数式で、組んだ高さとの差は 0.6rem ほど。
 */
export function displayMathHeight(tex: string): number {
  const rows = MULTI_ROW.test(tex) ? tex.split("\\\\").length : 1;
  return 2 + rows * (TALL.test(tex) ? 2.9 : 1.75);
}

/** 文中で大きく組む記法（\dfrac など）。文中の \frac や \sum は小さく組むので、行の高さに収まる。 */
const INLINE_TALL = /\\(?:dfrac|dbinom|displaystyle)(?![A-Za-z])/;

/** 組む前の数式に付ける、組んだ後と同じだけ場所を取るための指定（ADR 0051）。 */
function placeholderStyle(tex: string, display: boolean) {
  if (display) return { minHeight: `${displayMathHeight(tex)}rem` };
  // 文中の大きな分数は、行を 2.5 文字分の高さに広げる。組む前から同じ高さにしておく。
  return INLINE_TALL.test(tex) ? { lineHeight: 2.5 } : undefined;
}

/**
 * 数式。KaTeX を読み込むまでは TeX の文字列をそのまま表示する。
 * 書き誤りがあれば、TeX の文字列と誤りがあることを表示する。
 */
export function MathView({ tex, display }: { tex: string; display: boolean }) {
  // 数式が画面の近くに来たときに、初めて KaTeX を読み込む（ADR 0038）。
  const [ref, near] = useNearViewport<HTMLElement>();
  const katex = useLazy(katexLoader, undefined, near);
  // 数式ごとに別のタスクで組む（ADR 0046）。読み込み終わったときに、画面の近くの数式をまとめて一度に組むと、長いタスクになる。
  // 読み込み済みで、初めから画面の近くにあるとき（サーバーでの描画）だけ、最初の描画で組む。
  // ブラウザでは Worker で組み（ADR 0053）、結果を数式ごとに別のタスクで描く。
  const [rendered, setRendered] = useState<{ tex: string; display: boolean; html: string | null } | null>(() => {
    if (!katex || !near) return null;
    const html = katex.renderMath(tex, display);
    return html instanceof Promise ? null : { tex, display, html };
  });
  const done = rendered?.tex === tex && rendered.display === display;
  useEffect(() => {
    if (done || !katex || !near) return;
    let cancelled = false;
    const cancelTurn = inTurn(() => {
      void Promise.resolve(katex.renderMath(tex, display)).then((html) => !cancelled && setRendered({ tex, display, html }));
    });
    return () => {
      cancelled = true;
      cancelTurn();
    };
  }, [done, katex, near, tex, display]);
  const Tag = display ? "div" : "span";
  const className = display ? MATH_DISPLAY_CLASS : MATH_INLINE_CLASS;
  if (!done) {
    return (
      <Tag ref={ref} className={className} style={placeholderStyle(tex, display)}>
        {tex}
      </Tag>
    );
  }

  const { html } = rendered;
  if (html === null) {
    return (
      <Tag ref={ref} className={`${className} math-error`}>
        <code>{tex}</code>
        <span className="math-error-message">（数式を解釈できませんでした）</span>
      </Tag>
    );
  }
  return <Tag ref={ref} className={className} dangerouslySetInnerHTML={{ __html: html }} />;
}
