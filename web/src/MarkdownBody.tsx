import { createContext, useContext, useEffect, useState } from "react";
import Markdown, { type Components, type ExtraProps, type Options } from "react-markdown";
import rehypeRaw from "rehype-raw";
import rehypeSanitize from "rehype-sanitize";
import remarkGfm from "remark-gfm";
import { highlightLoader, type Highlight } from "./highlightLoader";
import { sanitizeSchema } from "./sanitize";
import { Link } from "./router";

const remarkPlugins: Options["remarkPlugins"] = [remarkGfm];

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
const baseRehypePlugins: NonNullable<Options["rehypePlugins"]> = [rehypeRaw, [rehypeSanitize, sanitizeSchema]];

/** シンタックスハイライトを読み込み、読み込み終わったら描き直す。読み込むまでコードブロックは色なし。 */
function useHighlight(): Highlight | null {
  const [highlight, setHighlight] = useState(highlightLoader.loaded);
  useEffect(() => {
    if (highlight) return;
    let mounted = true;
    highlightLoader.load().then(
      (loaded) => mounted && setHighlight(loaded),
      (error: unknown) => console.error("シンタックスハイライトを読み込めませんでした", error),
    );
    return () => {
      mounted = false;
    };
  }, [highlight]);
  return highlight;
}

function isInternal(href: string | undefined): href is string {
  return href !== undefined && href.startsWith("/") && !href.startsWith("//");
}

type HastNode = { type: string; value?: string; tagName?: string; children?: HastNode[] };

/** 項目の文章。入れ子のリストの文章は含めない。 */
function textOf(node: HastNode): string {
  if (node.type === "text") return node.value ?? "";
  if (node.tagName === "ul" || node.tagName === "ol") return "";
  return (node.children ?? []).map(textOf).join("");
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
  input({ node: _node, ...props }) {
    const label = useContext(TaskLabel);
    return <input {...props} aria-label={props.type === "checkbox" ? label : undefined} />;
  },
};

export function MarkdownBody({ source, className = "mt-6" }: { source: string; className?: string }) {
  const highlight = useHighlight();
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
