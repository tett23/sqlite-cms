import { useEffect, useState } from "react";
import { inTurn, useLazy } from "../lazyLoader";
import { useNearViewport } from "../nearViewport";
import { MATH_DISPLAY_CLASS, MATH_INLINE_CLASS } from "../markdown/math";
import { katexLoader } from "./loaders";

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
  const [rendered, setRendered] = useState<{ tex: string; display: boolean; html: string | null } | null>(() =>
    katex && near ? { tex, display, html: katex.renderMath(tex, display) } : null,
  );
  const done = rendered?.tex === tex && rendered.display === display;
  useEffect(() => {
    if (done || !katex || !near) return;
    return inTurn(() => setRendered({ tex, display, html: katex.renderMath(tex, display) }));
  }, [done, katex, near, tex, display]);
  const Tag = display ? "div" : "span";
  const className = display ? MATH_DISPLAY_CLASS : MATH_INLINE_CLASS;
  if (!done) {
    return (
      <Tag ref={ref} className={className}>
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
