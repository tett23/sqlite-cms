import { toChildArray } from "preact";
import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import Markdown, { type Components, type ExtraProps, type Options } from "react-markdown";
import rehypeSanitize from "rehype-sanitize";
import { withBasePath } from "./base";
import type { HighlightedBlock } from "./highlight";
import { highlightLoader } from "./highlightLoader";
import { needsHighlight } from "./highlightLanguages";
import { inTurn, useLazy } from "./lazyLoader";
import { useNearViewport } from "./nearViewport";
import { MATH_DISPLAY_CLASS, MATH_INLINE_CLASS } from "./markdown/math";
import { firstPart } from "./markdown/firstPart";
import { remarkGfmParse } from "./markdown/gfm";
import { LINK_CARD_SLOT_CLASS } from "./markdown/transforms";
import { linkCardLoader } from "./render/loaders";
import { Diagram } from "./render/Diagram";
import { MathView } from "./render/Math";
import { rehypeRawHtml } from "./markdown/rawHtml";
import { remarkExtensions } from "./markdown/remarkExtensions";
import { PARTIAL_SEARCH_CLASS, sanitizeSchema, unwrapPlainElements } from "./sanitize";
import { EXTERNAL_LINK_PROPS, isExternal } from "./externalLink";
import { Link } from "./router";
import type { MediaSize } from "./db";

const NO_LINK_CARD_IMAGES: ReadonlyMap<string, string> = new Map();
const NO_MEDIA_SIZES: ReadonlyMap<string, MediaSize> = new Map();
const NO_PARTIALS: PartialComponents = {};

const remarkPlugins: Options["remarkPlugins"] = [remarkGfmParse, remarkExtensions];

const remarkRehypeOptions: Options["remarkRehypeOptions"] = {
  allowDangerousHtml: true,
  footnoteLabel: "脚注",
  footnoteLabelProperties: {},
  // 読み上げ用の名前に、見えている記号を含める（見た目と読み上げを一致させる）。
  footnoteBackLabel: "↩︎ 本文に戻る",
  footnoteBackContent: "↩︎",
};

// 本文の HTML を取り込み（自前の rehypeRawHtml、ADR 0046）、許可した要素と属性だけを残す（sanitize）。
// Shiki の色分けは、ここではなく pre の部品で行う。sanitize の後なので、Shiki が付けた色の指定は取り除かれない。
const baseRehypePlugins: NonNullable<Options["rehypePlugins"]> = [
  rehypeRawHtml,
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

/** 本文の画像（/media/…）の大きさ（DB の media_sizes、ADR 0051）。 */
const MediaSizes = createContext<ReadonlyMap<string, MediaSize>>(new Map());

/** 画像に付ける width と height。大きさが分かれば、画像を読み込む前から場所を取り、本文がずれないようにする（ADR 0051）。 */
function imageSize(src: unknown, width: unknown, sizes: ReadonlyMap<string, MediaSize>): { width?: number; height?: number } {
  if (typeof src !== "string") return {};
  let size = sizes.get(src);
  if (!size) {
    try {
      size = sizes.get(decodeURI(src));
    } catch {
      return {};
    }
  }
  if (!size) return {};
  // `=160x` で幅だけを指定したときは、縦横比を保つ高さにする。
  const shown = typeof width === "number" ? width : typeof width === "string" && /^\d+$/.test(width) ? Number(width) : null;
  if (shown === null) return { width: size.width, height: size.height };
  return { width: shown, height: Math.round((shown * size.height) / size.width) };
}

/** リンクカードの URL と画像のパス（DB の link_cards、ADR 0028）。 */
const LinkCardImages = createContext<ReadonlyMap<string, string>>(new Map());

const logLinkCardError = (error: unknown) => console.error("リンクカードを読み込めませんでした", error);

/**
 * リンクカードを置く場所（ADR 0049）。画面の近くに来たら、カードの部品を読み込んで描く。
 * 読み込むまでは中身のリンクを、カードと同じ枠の中に出す（CSS で、カードと同じ最低の高さを取る）。
 */
function LinkCardSlot({ url, children }: { url: string; children: ReactNode }) {
  const [ref, near] = useNearViewport<HTMLDivElement>();
  const cards = useLazy(linkCardLoader, logLinkCardError, near);
  const image = useContext(LinkCardImages).get(url);
  if (cards) return <cards.LinkCard url={url} image={image === undefined ? undefined : withBasePath(image)} />;
  return (
    <div ref={ref} className={LINK_CARD_SLOT_CLASS}>
      {children}
    </div>
  );
}

/** リンクカードを置く場所の、中身のリンクの URL。 */
function slotUrl(node: HastNode): string | null {
  const link = node.children?.find((child) => child.tagName === "a") as (HastNode & { properties?: { href?: unknown } }) | undefined;
  return typeof link?.properties?.href === "string" ? link.properties.href : null;
}

/** ヘッダの `{{> 名前}}` に置く部品（ADR 0043）。渡さなければ何も置かない。 */
export type PartialComponents = Readonly<Record<string, ReactNode>>;

const Partials = createContext<PartialComponents>({});

/** タスクリストのチェックボックスに、項目の文章を読み上げ用の名前として渡す。 */
const TaskLabel = createContext<string | undefined>(undefined);

const components: Components = {
  a({ node: _node, href, ...props }) {
    if (isInternal(href)) return <Link to={href} {...props} />;
    return <a href={href} {...props} {...(isExternal(href) ? EXTERNAL_LINK_PROPS : {})} />;
  },
  // 本文の画像（/media/…）とリンクカードの画像（/link-cards/…）は、サイトを置くパスから始める（ADR 0030）。
  img({ node: _node, src, ...props }) {
    const size = imageSize(src, props.width, useContext(MediaSizes));
    return <img src={typeof src === "string" ? withBasePath(src) : src} {...props} {...size} />;
  },
  li({ node, ...props }) {
    const item = <li {...props} />;
    return isTaskListItem(node) ? <TaskLabel value={textOf(node as HastNode).trim()}>{item}</TaskLabel> : item;
  },
  div({ node, ...props }) {
    const partials = useContext(Partials);
    if (hasClass(node as HastNode, MATH_DISPLAY_CLASS)) return <MathView tex={textOf(node as HastNode)} display />;
    if (hasClass(node as HastNode, PARTIAL_SEARCH_CLASS)) return <>{partials.search ?? null}</>;
    const url = hasClass(node as HastNode, LINK_CARD_SLOT_CLASS) ? slotUrl(node as HastNode) : null;
    if (url) return <LinkCardSlot url={url}>{props.children}</LinkCardSlot>;
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
    const [highlighted, setHighlighted] = useState<{ key: string; block: HighlightedBlock | null } | null>(() => {
      if (!canHighlight) return null;
      const block = highlight.highlightBlock(source, languageClass);
      return block instanceof Promise ? null : { key, block };
    });
    const done = highlighted?.key === key;
    useEffect(() => {
      if (done || !canHighlight) return;
      let cancelled = false;
      // 色分けの結果（Worker なら届いた結果）を、ブロックごとに別のタスクで描く（ADR 0038、0046）。
      const cancelTurn = inTurn(() => {
        void Promise.resolve(highlight.highlightBlock(source, languageClass)).then(
          (block) => !cancelled && setHighlighted({ key, block }),
          logHighlightError,
        );
      });
      return () => {
        cancelled = true;
        cancelTurn();
      };
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

/** 長い本文を少しずつ描くとき（ADR 0051）、最初に描く要素の数（ブロックの間の改行を含む）。最初の画面に収まるより多くする。 */
const FIRST_CHILDREN = 24;
/** 残りを描くときに、一度に描く要素の数。 */
const NEXT_CHILDREN = 48;

/** GFM の脚注の欄（section.footnotes）。 */
function isFootnotes(child: unknown): boolean {
  const vnode = child as { type?: unknown; props?: { className?: unknown } } | null;
  return typeof vnode === "object" && vnode?.type === "section" && vnode.props?.className === "footnotes";
}

/** 描いた結果が画面に出た後に、fn を別のタスクで呼ぶ。取り消す関数を返す。 */
function afterPaint(fn: () => void): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const frame = requestAnimationFrame(() => {
    timer = setTimeout(fn, 0);
  });
  return () => {
    cancelAnimationFrame(frame);
    clearTimeout(timer);
  };
}

export function MarkdownBody({
  source,
  className = "mt-6",
  baseClassName = "article-body",
  linkCardImages = NO_LINK_CARD_IMAGES,
  mediaSizes = NO_MEDIA_SIZES,
  partials = NO_PARTIALS,
  progressive = false,
}: {
  source: string;
  className?: string;
  /** 見た目の種類。本文は article-body、ヘッダは site-header-body（ADR 0043）。 */
  baseClassName?: string;
  /** リンクカードの URL と画像のパス（DB の link_cards、ADR 0028）。 */
  linkCardImages?: ReadonlyMap<string, string>;
  /** 本文の画像の大きさ（DB の media_sizes、ADR 0051）。 */
  mediaSizes?: ReadonlyMap<string, MediaSize>;
  /** `{{> 名前}}` に置く部品（ヘッダだけで使う）。 */
  partials?: PartialComponents;
  /**
   * 少しずつ描く（ADR 0051）。最初は先頭の部分の、先頭の要素だけを解析して描き、画面に出す。
   * 全体の解析と残りの要素は、画面に出た後に、別のタスクで少しずつ描く。
   * 長い本文を一度に解析して描くと、最初の画面に出るのが遅れ（LCP）、長いタスクになるためである。
   */
  progressive?: boolean;
}) {
  // 少しずつ描くときは、最初の画面には先頭の部分（firstPart）だけを解析して描く。全体の解析は、画面に出た後に行う。
  const part = useMemo(() => (progressive ? firstPart(source) : null), [progressive, source]);
  const initial = { source, whole: part === null, count: progressive ? FIRST_CHILDREN : Infinity };
  const [progress, setProgress] = useState(initial);
  const current = progress.source === source ? progress : initial;
  // Markdown を解析して要素を作るのは、描く文字列が変わったときだけにする。少しずつ描くときも、作った要素を使い回す。
  const text = current.whole ? source : part!;
  const children = useMemo(() => {
    const all = toChildArray(
      Markdown({ remarkPlugins, remarkRehypeOptions, rehypePlugins: baseRehypePlugins, components, children: text }).props.children,
    );
    // 先頭の部分の脚注の欄は描かない。全体を描くと本文の最後に移り、それまでに画面に出ていれば、ずれになる。
    return current.whole ? all : all.filter((child) => !isFootnotes(child));
  }, [text, current.whole]);
  const done = current.whole && current.count >= children.length;
  useEffect(() => {
    if (done) return;
    return afterPaint(() =>
      setProgress({ source, whole: true, count: current.whole ? current.count + NEXT_CHILDREN : current.count }),
    );
  }, [done, source, current.whole, current.count]);
  const count = current.count;
  return (
    <div className={className ? `${baseClassName} ${className}` : baseClassName}>
      <Partials value={partials}>
        <LinkCardImages value={linkCardImages}>
          <MediaSizes value={mediaSizes}>{count >= children.length ? children : children.slice(0, count)}</MediaSizes>
        </LinkCardImages>
      </Partials>
    </div>
  );
}
