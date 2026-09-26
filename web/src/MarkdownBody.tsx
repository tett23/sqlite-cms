import { createContext, useContext, useMemo } from "react";
import Markdown, { type Components, type ExtraProps, type Options } from "react-markdown";
import rehypeRaw from "rehype-raw";
import rehypeSanitize from "rehype-sanitize";
import remarkGfm from "remark-gfm";
import { highlightLoader } from "./highlightLoader";
import { needsHighlight } from "./highlightLanguages";
import { useLazy, useLoaded } from "./lazyLoader";
import { MATH_DISPLAY_CLASS, MATH_INLINE_CLASS } from "./markdown/math";
import { Diagram } from "./render/Diagram";
import { MathView } from "./render/Math";
import { remarkExtensions } from "./markdown/remarkExtensions";
import { sanitizeSchema, unwrapPlainElements } from "./sanitize";
import { Link } from "./router";

const NO_LINK_CARD_IMAGES: ReadonlyMap<string, string> = new Map();

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
    const isCode = code?.tagName === "code";
    // 色を付けられる言語のコードブロックが表示されたときに、初めて Shiki を読み込む（ADR 0029）。
    // 読み込むまでは色なしで表示し、読み込み終わったら MarkdownBody が描き直す。
    useLazy(highlightLoader, logHighlightError, isCode && needsHighlight(code.properties?.className));
    if (isCode && hasClass(code, "language-mermaid")) {
      return <Diagram source={textOf(code).replace(/\n$/, "")} />;
    }
    return <pre {...props} />;
  },
  input({ node: _node, ...props }) {
    const label = useContext(TaskLabel);
    return <input {...props} aria-label={props.type === "checkbox" ? label : undefined} />;
  },
};

export function MarkdownBody({
  source,
  className = "mt-6",
  linkCardImages = NO_LINK_CARD_IMAGES,
}: {
  source: string;
  className?: string;
  /** リンクカードの URL と画像のパス（DB の link_cards、ADR 0028）。 */
  linkCardImages?: ReadonlyMap<string, string>;
}) {
  // 読み込みはコードブロックの部品が始める。ここでは読み込み終わりを待って描き直すだけにする。
  const highlight = useLoaded(highlightLoader);
  const remarkPlugins = useMemo<Options["remarkPlugins"]>(
    () => [remarkGfm, [remarkExtensions, { linkCardImage: (url: string) => linkCardImages.get(url) }]],
    [linkCardImages],
  );
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
