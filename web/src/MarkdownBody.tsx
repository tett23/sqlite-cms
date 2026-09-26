import { createContext, useContext, useEffect, useMemo, useState, type ComponentProps, type ReactNode } from "react";
import Markdown, { type Components, type ExtraProps, type Options } from "react-markdown";
import rehypeRaw from "rehype-raw";
import rehypeSanitize from "rehype-sanitize";
import remarkGfm from "remark-gfm";
import { withBasePath } from "./base";
import type { HighlightedBlock } from "./highlight";
import { highlightLoader } from "./highlightLoader";
import { needsHighlight } from "./highlightLanguages";
import { inTurn, useLazy } from "./lazyLoader";
import { useNearViewport } from "./nearViewport";
import { MATH_DISPLAY_CLASS, MATH_INLINE_CLASS } from "./markdown/math";
import { LINK_CARD_IMAGE_CLASS } from "./markdown/transforms";
import { Diagram } from "./render/Diagram";
import { MathView } from "./render/Math";
import { remarkExtensions } from "./markdown/remarkExtensions";
import { PARTIAL_SEARCH_CLASS, sanitizeSchema, unwrapPlainElements } from "./sanitize";
import { Link } from "./router";

const NO_LINK_CARD_IMAGES: ReadonlyMap<string, string> = new Map();
const NO_PARTIALS: PartialComponents = {};

const remarkRehypeOptions: Options["remarkRehypeOptions"] = {
  allowDangerousHtml: true,
  footnoteLabel: "脚注",
  footnoteLabelProperties: {},
  // 読み上げ用の名前に、見えている記号を含める（見た目と読み上げを一致させる）。
  footnoteBackLabel: "↩︎ 本文に戻る",
  footnoteBackContent: "↩︎",
};

// 本文の HTML を取り込み（raw）、許可した要素と属性だけを残す（sanitize）。
// Shiki の色分けは、ここではなく pre の部品で行う。sanitize の後なので、Shiki が付けた色の指定は取り除かれない。
const baseRehypePlugins: NonNullable<Options["rehypePlugins"]> = [
  rehypeRaw,
  [rehypeSanitize, sanitizeSchema],
  unwrapPlainElements,
];

const logHighlightError = (error: unknown) => console.error("シンタックスハイライトを読み込めませんでした", error);

function isInternal(href: unknown): href is string {
  return typeof href === "string" && href.startsWith("/") && !href.startsWith("//");
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

/**
 * リンクカードの画像は、画面の近くに来てから読み込む（ADR 0038）。
 * loading="lazy" だけでは、Chrome は画面から 1,250 px 以内の画像をすぐに読み込むので、画面の外のカードの画像が本文の表示と回線を取り合う。
 * 画像の大きさは CSS で固定してあるので、後から読み込んでも本文はずれない。
 */
function LinkCardImage({ src, ...props }: ComponentProps<"img">) {
  const [ref, near] = useNearViewport<HTMLImageElement>();
  return <img ref={ref} src={near ? src : undefined} {...props} />;
}

/** ヘッダの `{{> 名前}}` に置く部品（ADR 0043）。渡さなければ何も置かない。 */
export type PartialComponents = Readonly<Record<string, ReactNode>>;

const Partials = createContext<PartialComponents>({});

/** タスクリストのチェックボックスに、項目の文章を読み上げ用の名前として渡す。 */
const TaskLabel = createContext<string | undefined>(undefined);

const components: Components = {
  a({ node: _node, href, ...props }) {
    return isInternal(href) ? <Link to={href} {...props} /> : <a href={href} {...props} />;
  },
  // 本文の画像（/media/…）とリンクカードの画像（/link-cards/…）は、サイトを置くパスから始める（ADR 0030）。
  img({ node, src, ...props }) {
    const path = typeof src === "string" ? withBasePath(src) : src;
    if (hasClass(node as HastNode, LINK_CARD_IMAGE_CLASS)) return <LinkCardImage src={path} {...props} />;
    return <img src={path} {...props} />;
  },
  li({ node, ...props }) {
    const item = <li {...props} />;
    return isTaskListItem(node) ? <TaskLabel value={textOf(node as HastNode).trim()}>{item}</TaskLabel> : item;
  },
  div({ node, ...props }) {
    const partials = useContext(Partials);
    if (hasClass(node as HastNode, MATH_DISPLAY_CLASS)) return <MathView tex={textOf(node as HastNode)} display />;
    if (hasClass(node as HastNode, PARTIAL_SEARCH_CLASS)) return <>{partials.search ?? null}</>;
    return <div {...props} />;
  },
  span({ node, ...props }) {
    if (hasClass(node as HastNode, MATH_INLINE_CLASS)) return <MathView tex={textOf(node as HastNode)} display={false} />;
    return <span {...props} />;
  },
  pre({ node, ...props }) {
    const code = (node as HastNode | undefined)?.children?.[0];
    const isCode = code?.tagName === "code";
    const className = isCode ? code.properties?.className : undefined;
    const languageClass = Array.isArray(className)
      ? (className.find((c) => typeof c === "string" && c.startsWith("language-")) as string | undefined)
      : undefined;
    const highlightable = isCode && needsHighlight(className);
    const source = isCode ? textOf(code) : "";
    // 色を付けられる言語のコードブロックが画面の近くに来たときに、初めて Shiki を読み込み、そのブロックだけを色付けする（ADR 0029、0038）。
    // 読み込むまでは色なしで表示する。本文の Markdown 全体は描き直さない。
    const [ref, near] = useNearViewport<HTMLPreElement>();
    const highlight = useLazy(highlightLoader, logHighlightError, near && highlightable);
    // 色分けは、ブロックごとに別のタスクで行う（inTurn）。読み込み済みで、初めから画面の近くにあるとき（サーバーでの描画）だけ、最初の描画で行う。
    // 同じ部品が別の記事のコードブロックに使い回されることがあるので、色分けの結果は、言語とコードの組と一緒に持つ。
    const key = `${languageClass}\n${source}`;
    const canHighlight = highlight !== null && near && highlightable && languageClass !== undefined;
    const [highlighted, setHighlighted] = useState<{ key: string; block: HighlightedBlock | null } | null>(() =>
      canHighlight ? { key, block: highlight.highlightBlock(source, languageClass) } : null,
    );
    const done = highlighted?.key === key;
    useEffect(() => {
      if (done || !canHighlight) return;
      return inTurn(() => setHighlighted({ key, block: highlight.highlightBlock(source, languageClass) }));
    }, [done, canHighlight, highlight, key, source, languageClass]);
    const block = done ? highlighted.block : null;
    if (isCode && hasClass(code, "language-mermaid")) {
      return <Diagram source={source.replace(/\n$/, "")} />;
    }
    if (block) {
      return <pre ref={ref} className={block.className} style={block.style} tabIndex={0} dangerouslySetInnerHTML={{ __html: block.html }} />;
    }
    return <pre ref={ref} {...props} />;
  },
  input({ node: _node, ...props }) {
    const label = useContext(TaskLabel);
    return <input {...props} aria-label={props.type === "checkbox" ? label : undefined} />;
  },
};

export function MarkdownBody({
  source,
  className = "mt-6",
  baseClassName = "article-body",
  linkCardImages = NO_LINK_CARD_IMAGES,
  partials = NO_PARTIALS,
}: {
  source: string;
  className?: string;
  /** 見た目の種類。本文は article-body、ヘッダは site-header-body（ADR 0043）。 */
  baseClassName?: string;
  /** リンクカードの URL と画像のパス（DB の link_cards、ADR 0028）。 */
  linkCardImages?: ReadonlyMap<string, string>;
  /** `{{> 名前}}` に置く部品（ヘッダだけで使う）。 */
  partials?: PartialComponents;
}) {
  const remarkPlugins = useMemo<Options["remarkPlugins"]>(
    () => [remarkGfm, [remarkExtensions, { linkCardImage: (url: string) => linkCardImages.get(url) }]],
    [linkCardImages],
  );
  return (
    <div className={className ? `${baseClassName} ${className}` : baseClassName}>
      <Partials value={partials}>
        <Markdown
          remarkPlugins={remarkPlugins}
          remarkRehypeOptions={remarkRehypeOptions}
          rehypePlugins={baseRehypePlugins}
          components={components}
        >
          {source}
        </Markdown>
      </Partials>
    </div>
  );
}
