import { useLazy } from "../lazyLoader";
import { MATH_DISPLAY_CLASS, MATH_INLINE_CLASS } from "../markdown/math";
import { katexLoader } from "./loaders";

/**
 * 数式。KaTeX を読み込むまでは TeX の文字列をそのまま表示する。
 * 書き誤りがあれば、TeX の文字列と誤りがあることを表示する。
 */
export function MathView({ tex, display }: { tex: string; display: boolean }) {
  const katex = useLazy(katexLoader);
  const Tag = display ? "div" : "span";
  const className = display ? MATH_DISPLAY_CLASS : MATH_INLINE_CLASS;
  if (!katex) return <Tag className={className}>{tex}</Tag>;

  const html = katex.renderMath(tex, display);
  if (html === null) {
    return (
      <Tag className={`${className} math-error`}>
        <code>{tex}</code>
        <span className="math-error-message">（数式を解釈できませんでした）</span>
      </Tag>
    );
  }
  return <Tag className={className} dangerouslySetInnerHTML={{ __html: html }} />;
}
