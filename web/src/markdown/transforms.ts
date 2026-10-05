// 構文木（mdast）を書き換える記法（ADR 0025）。
import type { Code, FootnoteDefinition, Image, Link, Nodes, Parent, PhrasingContent, Root, RootContent, Text } from "mdast";
import type { CodeFigure } from "./types";

/** 子を持つノードをすべて訪ねる（子を書き換えてもよい）。 */
function eachParent(node: Nodes, visit: (parent: Parent) => void) {
  if (!("children" in node)) return;
  visit(node);
  for (const child of node.children) eachParent(child, visit);
}

/** diff と言語を同時に指定したコードブロックの言語名。`diff-<言語>` にする（highlight/diff.ts が読む）。 */
export const DIFF_LANGUAGE_PREFIX = "diff-";

/**
 * コードブロックの情報文字列を読む。
 * - `js:foo.js`：言語 js、ファイル名 foo.js
 * - `:foo.txt`：言語なし、ファイル名 foo.txt
 * - `diff js`、`diff js:foo.js`：diff と言語 js の色分けを同時に付ける
 */
export function parseCodeInfo(lang: string | null | undefined, meta: string | null | undefined) {
  let language = lang ?? "";
  let rest = meta ?? "";
  let diff = false;
  if (language === "diff" && rest !== "") {
    diff = true;
    const [first = "", ...others] = rest.split(/\s+/);
    language = first;
    rest = others.join(" ");
  }
  let filename = "";
  const colon = language.indexOf(":");
  if (colon >= 0) {
    filename = language.slice(colon + 1);
    language = language.slice(0, colon);
  }
  return { language, filename, diff, meta: rest };
}

/** コードブロックのファイル名を figure と figcaption にし、diff と言語の指定を言語名にまとめる。 */
export function transformCodeBlocks(tree: Root) {
  eachParent(tree, (parent) => {
    parent.children = parent.children.map((child) => {
      if (child.type !== "code") return child;
      const code: Code = child;
      const { language, filename, diff, meta } = parseCodeInfo(code.lang, code.meta);
      code.lang = diff ? (language ? `${DIFF_LANGUAGE_PREFIX}${language}` : "diff") : language || null;
      code.meta = meta || null;
      if (!filename) return code;
      const figure: CodeFigure = {
        type: "codeFigure",
        data: { hName: "figure" },
        children: [
          { type: "codeFigureCaption", data: { hName: "figcaption" }, children: [{ type: "text", value: filename }] },
          code,
        ],
      };
      return figure as RootContent;
    }) as typeof parent.children;
  });
}

const INLINE_FOOTNOTE_OPEN = "^[";

/** 閉じの ] の位置。[ と ] の対応を数える。見つからなければ -1。 */
function findClosingBracket(value: string, from: number, depth: { value: number }): number {
  for (let i = from; i < value.length; i++) {
    if (value[i] === "[") depth.value++;
    else if (value[i] === "]" && --depth.value === 0) return i;
  }
  return -1;
}

/**
 * インラインの脚注 `^[内容]` を、GFM の脚注と同じ参照と定義にする。
 * 内容には強調やリンクを書ける。定義は文書の末尾に足し、番号は参照の順に振られる。
 */
export function transformInlineFootnotes(tree: Root) {
  const used = new Set<string>();
  eachParent(tree, (parent) => {
    for (const child of parent.children) {
      if (child.type === "footnoteDefinition") used.add(child.identifier.toLowerCase());
    }
  });
  let count = 0;
  const nextIdentifier = () => {
    let identifier: string;
    do identifier = `inline-${++count}`;
    while (used.has(identifier));
    return identifier;
  };
  const definitions: FootnoteDefinition[] = [];

  function transform(parent: Parent) {
    const children = parent.children as PhrasingContent[];
    for (let i = 0; i < children.length; i++) {
      const start = children[i];
      if (start.type !== "text") continue;
      const open = start.value.indexOf(INLINE_FOOTNOTE_OPEN);
      if (open < 0) continue;

      const depth = { value: 1 };
      let endIndex = i;
      let close = findClosingBracket(start.value, open + INLINE_FOOTNOTE_OPEN.length, depth);
      while (close < 0 && ++endIndex < children.length) {
        const node = children[endIndex];
        if (node.type === "text") close = findClosingBracket(node.value, 0, depth);
      }
      if (close < 0) continue;

      const end = children[endIndex] as Text;
      const content: PhrasingContent[] =
        endIndex === i
          ? [{ type: "text", value: start.value.slice(open + INLINE_FOOTNOTE_OPEN.length, close) }]
          : [
              { type: "text", value: start.value.slice(open + INLINE_FOOTNOTE_OPEN.length) },
              ...children.slice(i + 1, endIndex),
              { type: "text", value: end.value.slice(0, close) },
            ];
      const identifier = nextIdentifier();
      const paragraph: Parent = { type: "paragraph", children: content.filter((n) => n.type !== "text" || n.value !== "") } as never;
      transform(paragraph);
      definitions.push({ type: "footnoteDefinition", identifier, label: identifier, children: [paragraph as never] });

      const replacement: PhrasingContent[] = [
        { type: "text", value: start.value.slice(0, open) },
        { type: "footnoteReference", identifier, label: identifier },
        { type: "text", value: end.value.slice(close + 1) },
      ].filter((n) => n.type !== "text" || n.value !== "") as PhrasingContent[];
      children.splice(i, endIndex - i + 1, ...replacement);
      // 置き換えた後ろの文字列にも ^[ があれば続けて読む。
      i += replacement.length - 2;
    }
  }

  eachParent(tree, transform);
  tree.children.push(...definitions);
}

const IMAGE_SIZE = /!\[([^[\]]*)\]\(([^\s()]+) =(\d+)x\)/g;
const IMAGE_SIZE_HEAD = /!\[([^[\]]*)\]\($/;
const IMAGE_SIZE_TAIL = /^ =(\d+)x\)/;

function image(url: string, alt: string, width: string): Image {
  return { type: "image", url, alt, data: { hProperties: { width: Number(width) } } };
}

/**
 * 画像の幅の指定 `![alt](URL =250x)` を読む（幅は px）。
 * CommonMark では URL の後の ` =250x` のために画像として読まれず、文字列として残る。
 * URL が http(s):// で始まると GFM の自動リンクになるので、文字列、リンク、文字列の並びも読む。
 */
export function transformImageSize(tree: Root) {
  eachParent(tree, (parent) => {
    const children = parent.children as PhrasingContent[];
    for (let i = 0; i < children.length; i++) {
      const node = children[i];
      if (node.type !== "text") continue;

      const link = children[i + 1];
      const tail = children[i + 2];
      const head = node.value.match(IMAGE_SIZE_HEAD);
      if (head && link?.type === "link" && isAutolink(link) && tail?.type === "text") {
        const size = tail.value.match(IMAGE_SIZE_TAIL);
        if (size) {
          const replacement = [
            { type: "text", value: node.value.slice(0, head.index) },
            image(link.url, head[1], size[1]),
            { type: "text", value: tail.value.slice(size[0].length) },
          ].filter((n) => n.type !== "text" || (n as Text).value !== "") as PhrasingContent[];
          children.splice(i, 3, ...replacement);
          i--;
          continue;
        }
      }

      const parts: PhrasingContent[] = [];
      let last = 0;
      for (const match of node.value.matchAll(IMAGE_SIZE)) {
        if (match.index > last) parts.push({ type: "text", value: node.value.slice(last, match.index) });
        parts.push(image(match[2], match[1], match[3]));
        last = match.index + match[0].length;
      }
      if (parts.length === 0) continue;
      if (last < node.value.length) parts.push({ type: "text", value: node.value.slice(last) });
      children.splice(i, 1, ...parts);
      i += parts.length - 1;
    }
  });
}

/** 文字列が URL そのものであるリンク（http(s):// で始まる GFM の自動リンクと <URL>）。 */
function isAutolink(link: Link): boolean {
  const [text] = link.children;
  return link.children.length === 1 && text.type === "text" && text.value === link.url;
}

/** リンクカードに付けるクラス。sanitize で許可する。 */
export const LINK_CARD_CLASS = "link-card";
export const LINK_CARD_HOST_CLASS = "link-card-host";
export const LINK_CARD_URL_CLASS = "link-card-url";
export const LINK_CARD_TEXT_CLASS = "link-card-text";
export const LINK_CARD_IMAGE_CLASS = "link-card-image";

/** リンクカードを置く場所に付けるクラス（ADR 0049）。sanitize で許可する。 */
export const LINK_CARD_SLOT_CLASS = "link-card-slot";

/**
 * URL だけの段落を、リンクカードを置く場所（`div.link-card-slot`）にする（ADR 0028、0049）。中身のリンクはそのまま残す。
 * カードは、置く場所が画面の近くに来てから読み込む部品（`web/src/render/LinkCard.tsx`）が描く。読み込むまでは、中身のリンクを出す。
 */
export function markLinkCards(tree: Root) {
  eachParent(tree, (parent) => {
    for (const child of parent.children) {
      if (child.type !== "paragraph") continue;
      const nodes = child.children.filter((n) => n.type !== "text" || n.value.trim() !== "");
      const [link] = nodes;
      if (nodes.length !== 1 || link.type !== "link" || !isAutolink(link)) continue;
      let url: URL;
      try {
        url = new URL(link.url);
      } catch {
        continue;
      }
      if (url.protocol !== "http:" && url.protocol !== "https:") continue;
      child.data = { hName: "div", hProperties: { className: [LINK_CARD_SLOT_CLASS] } };
      child.children = [link];
    }
  });
}
