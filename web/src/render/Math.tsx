import { useLazy } from "../lazyLoader";
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
  const Tag = display ? "div" : "span";
  const className = display ? MATH_DISPLAY_CLASS : MATH_INLINE_CLASS;
  if (!katex) {
    return (
      <Tag ref={ref} className={className}>
        {tex}
      </Tag>
    );
  }

  const html = katex.renderMath(tex, display);
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
