// 本文に書いた HTML（raw のノード）を、要素として取り込む rehype のプラグイン（ADR 0046）。
// rehype-raw の代わりに自前で書く。rehype-raw は HTML パーサ（parse5）を持ち込み、本体の JS を gzip 後で約 52 KB 大きくしていた。
//
// HTML の仕様どおりの解析はせず、本文に書く程度の HTML（ADR 0018 で許可したタグ）を正しく取り込めればよいものとする。
// 取り込んだ後に必ず rehype-sanitize を通すので、崩れた HTML を仕様と違う形に取り込んでも、許可しない要素や属性は残らない。
//
// - raw のノードを、開始タグ、終了タグ、文字、注釈に分ける。注釈、<!doctype>、CDATA は捨てる。
// - 兄弟のノードを順に見て、開いている要素を積み上げ、その後のノードを子にする。
//   <details> の開始と終了の間に Markdown の段落を挟む書き方も、一つの要素にまとまる（rehype-raw と同じ）。
// - 閉じていない要素は、親の終わりで閉じる。開いていない要素の終了タグは捨てる。
// - script、style、textarea、title の中は、タグとして読まない。
import { decodeNamedCharacterReference } from "decode-named-character-reference";
import type { Element, ElementContent, Nodes, Parents, Root, RootContent, Text } from "hast";
import { h } from "hastscript";

/** 終了タグを持たない要素。 */
const VOID = new Set(["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"]);

/** 中をタグとして読まない要素。 */
const RAW_TEXT = new Set(["script", "style", "textarea", "title", "xmp", "iframe", "noembed", "noframes", "plaintext"]);

type Token =
  | { type: "start"; tagName: string; attributes: Record<string, string>; selfClosing: boolean }
  | { type: "end"; tagName: string }
  | { type: "text"; value: string };

/** 文字参照（&amp; &#39; &#x3C; など）を文字にする。知らない名前はそのまま残す。 */
export function decodeEntities(text: string): string {
  return text.replace(/&(#[xX][0-9a-fA-F]+|#[0-9]+|[A-Za-z][A-Za-z0-9]*);/g, (match, reference: string) => {
    if (reference.startsWith("#")) {
      const code = reference[1] === "x" || reference[1] === "X" ? parseInt(reference.slice(2), 16) : parseInt(reference.slice(1), 10);
      // 0、サロゲート、Unicode の範囲外は、置換文字にする（HTML の仕様と同じ）。
      if (code === 0 || (code >= 0xd800 && code <= 0xdfff) || code > 0x10ffff) return "�";
      return String.fromCodePoint(code);
    }
    const decoded = decodeNamedCharacterReference(reference);
    return decoded === false ? match : decoded;
  });
}

const TAG_NAME = /^[A-Za-z][A-Za-z0-9-]*/;
const ATTRIBUTE = /^\s*([^\s"'>/=]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/;

/** HTML の文字列を、開始タグ、終了タグ、文字に分ける。 */
export function tokenize(html: string): Token[] {
  const tokens: Token[] = [];
  let pos = 0;
  const pushText = (value: string) => {
    if (value) tokens.push({ type: "text", value: decodeEntities(value) });
  };
  while (pos < html.length) {
    const lt = html.indexOf("<", pos);
    if (lt === -1) {
      pushText(html.slice(pos));
      break;
    }
    pushText(html.slice(pos, lt));
    const rest = html.slice(lt);
    // 注釈、<!doctype>、CDATA、処理命令は捨てる。
    if (rest.startsWith("<!--")) {
      const end = html.indexOf("-->", lt + 4);
      pos = end === -1 ? html.length : end + 3;
      continue;
    }
    if (rest.startsWith("<!") || rest.startsWith("<?")) {
      const end = html.indexOf(">", lt);
      pos = end === -1 ? html.length : end + 1;
      continue;
    }
    const isEnd = rest.startsWith("</");
    const name = rest.slice(isEnd ? 2 : 1).match(TAG_NAME);
    if (!name) {
      // タグではない < は文字として扱う。
      pushText("<");
      pos = lt + 1;
      continue;
    }
    const tagName = name[0].toLowerCase();
    let cursor = lt + (isEnd ? 2 : 1) + name[0].length;
    const attributes: Record<string, string> = {};
    for (;;) {
      const attribute = html.slice(cursor).match(ATTRIBUTE);
      if (!attribute) break;
      const key = attribute[1].toLowerCase();
      if (!(key in attributes)) attributes[key] = decodeEntities(attribute[2] ?? attribute[3] ?? attribute[4] ?? "");
      cursor += attribute[0].length;
    }
    const close = html.slice(cursor).match(/^\s*(\/?)>/);
    if (!close) {
      // 閉じていないタグは、文字として扱う。
      pushText(html.slice(lt, cursor));
      pos = cursor;
      continue;
    }
    pos = cursor + close[0].length;
    if (isEnd) {
      tokens.push({ type: "end", tagName });
      continue;
    }
    tokens.push({ type: "start", tagName, attributes, selfClosing: close[1] === "/" });
    if (RAW_TEXT.has(tagName)) {
      // 中はタグとして読まず、終了タグまでを文字にする。
      const end = html.toLowerCase().indexOf(`</${tagName}`, pos);
      const text = html.slice(pos, end === -1 ? html.length : end);
      if (text) tokens.push({ type: "text", value: text });
      if (end === -1) {
        pos = html.length;
      } else {
        const endClose = html.indexOf(">", end);
        pos = endClose === -1 ? html.length : endClose + 1;
        tokens.push({ type: "end", tagName });
      }
    }
  }
  return tokens;
}

/** 親の子を順に見て、raw のノードを取り込み、開いている要素の子にまとめ直す。 */
function rebuild(parent: Parents) {
  const children: RootContent[] = [];
  // 開いている要素。先頭は親そのもの（子を children に積む）。
  const stack: { tagName: string | null; children: (RootContent | ElementContent)[] }[] = [{ tagName: null, children }];
  const current = () => stack[stack.length - 1].children;

  for (const child of parent.children as RootContent[]) {
    if (child.type !== "raw") {
      if ("children" in child) rebuild(child as Parents);
      current().push(child);
      continue;
    }
    for (const token of tokenize(child.value)) {
      if (token.type === "text") {
        current().push({ type: "text", value: token.value } satisfies Text);
      } else if (token.type === "start") {
        const element = h(token.tagName, token.attributes) as Element;
        current().push(element);
        if (!VOID.has(token.tagName) && !token.selfClosing) stack.push({ tagName: token.tagName, children: element.children });
      } else {
        // 開いている同じ名前の要素まで閉じる。開いていなければ捨てる。
        for (let index = stack.length - 1; index > 0; index--) {
          if (stack[index].tagName === token.tagName) {
            stack.length = index;
            break;
          }
        }
      }
    }
  }
  parent.children = children as typeof parent.children;
}

/** rehype のプラグイン。 */
export function rehypeRawHtml() {
  return (tree: Root) => {
    rebuild(tree as Nodes as Parents);
  };
}
