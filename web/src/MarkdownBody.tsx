import { createContext, useContext } from "react";
import Markdown, { type Components, type ExtraProps, type Options } from "react-markdown";
import rehypeRaw from "rehype-raw";
import rehypeSanitize from "rehype-sanitize";
import remarkGfm from "remark-gfm";
import { highlightLoader } from "./highlightLoader";
import { useLazy } from "./lazyLoader";
import { MATH_DISPLAY_CLASS, MATH_INLINE_CLASS } from "./markdown/math";
import { Diagram } from "./render/Diagram";
import { MathView } from "./render/Math";
import { remarkExtensions } from "./markdown/remarkExtensions";
import { sanitizeSchema, unwrapPlainElements } from "./sanitize";
import { Link } from "./router";

const remarkPlugins: Options["remarkPlugins"] = [remarkGfm, remarkExtensions];

const remarkRehypeOptions: Options["remarkRehypeOptions"] = {
  allowDangerousHtml: true,
  footnoteLabel: "脚注",
  footnoteLabelProperties: {},
  // 読み上げ用の名前に、見えている記号を含める（見た目と読み上げを一致させる）。
  footnoteBackLabel: "↩︎ 本文に戻る",
  footnoteBackContent: "↩︎",
};

// 本文の HTML を取り込み（raw）、許可した要素と属性だけを残す（sanitize）。
// Shiki は sanitize の後に通す。先に通すと、Shiki が付けた色の指定が取り除かれる。
const baseRehypePlugins: NonNullable<Options["rehypePlugins"]> = [
  rehypeRaw,
  [rehypeSanitize, sanitizeSchema],
  unwrapPlainElements,
];

const logHighlightError = (error: unknown) => console.error("シンタックスハイライトを読み込めませんでした", error);

function isInternal(href: string | undefined): href is string {
  return href !== undefined && href.startsWith("/") && !href.startsWith("//");
}

type HastNode = {
  type: string;
  value?: string;
  tagName?: string;
  properties?: { className?: unknown };
  children?: HastNode[];
};

/** 項目の文章。入れ子のリストの文章は含めない。 */
function textOf(node: HastNode): string {
  if (node.type === "text") return node.value ?? "";
  if (node.tagName === "ul" || node.tagName === "ol") return "";
  return (node.children ?? []).map(textOf).join("");
}

function hasClass(node: HastNode | undefined, name: string): boolean {
  const className = node?.properties?.className;
  return Array.isArray(className) && className.includes(name);
}

function isTaskListItem(node: ExtraProps["node"]): boolean {
  const className = node?.properties?.className;
  return Array.isArray(className) && className.includes("task-list-item");
}

/** タスクリストのチェックボックスに、項目の文章を読み上げ用の名前として渡す。 */
const TaskLabel = createContext<string | undefined>(undefined);

const components: Components = {
  a({ node: _node, href, ...props }) {
    return isInternal(href) ? <Link to={href} {...props} /> : <a href={href} {...props} />;
  },
  li({ node, ...props }) {
    const item = <li {...props} />;
    return isTaskListItem(node) ? <TaskLabel value={textOf(node as HastNode).trim()}>{item}</TaskLabel> : item;
  },
  div({ node, ...props }) {
    if (hasClass(node as HastNode, MATH_DISPLAY_CLASS)) return <MathView tex={textOf(node as HastNode)} display />;
    return <div {...props} />;
  },
  span({ node, ...props }) {
    if (hasClass(node as HastNode, MATH_INLINE_CLASS)) return <MathView tex={textOf(node as HastNode)} display={false} />;
    return <span {...props} />;
  },
  pre({ node, ...props }) {
    const code = (node as HastNode | undefined)?.children?.[0];
    if (code?.tagName === "code" && hasClass(code, "language-mermaid")) {
      return <Diagram source={textOf(code).replace(/\n$/, "")} />;
    }
    return <pre {...props} />;
  },
  input({ node: _node, ...props }) {
    const label = useContext(TaskLabel);
    return <input {...props} aria-label={props.type === "checkbox" ? label : undefined} />;
  },
};

export function MarkdownBody({ source, className = "mt-6" }: { source: string; className?: string }) {
  const highlight = useLazy(highlightLoader, logHighlightError);
  return (
    <div className={`article-body ${className}`}>
      <Markdown
        remarkPlugins={remarkPlugins}
        remarkRehypeOptions={remarkRehypeOptions}
        rehypePlugins={highlight ? [...baseRehypePlugins, ...(highlight.rehypePlugins ?? [])] : baseRehypePlugins}
        components={components}
      >
        {source}
      </Markdown>
    </div>
  );
}
