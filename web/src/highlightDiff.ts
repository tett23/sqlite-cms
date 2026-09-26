// diff と言語の色分けを同時に付ける（```diff js、ADR 0025）。Shiki の前に通す。
import type { Element, ElementContent, Root, RootContent, Text } from "hast";
import type { HighlighterCore } from "shiki/core";
import { DIFF_LANGUAGE_PREFIX } from "./markdown/transforms";

const LANGUAGE_CLASS = `language-${DIFF_LANGUAGE_PREFIX}`;

export interface DiffOptions {
  theme: string;
  colorReplacements?: Record<string, string>;
}

/** 行頭の記号と、行に付けるクラス。+ と > は追加、- と < は削除、空白は変更なし。 */
const MARKERS: Record<string, string | null> = {
  "+": "diff-add",
  ">": "diff-add",
  "-": "diff-remove",
  "<": "diff-remove",
  " ": null,
};

function textOf(node: ElementContent | RootContent): string {
  if (node.type === "text") return node.value;
  return "children" in node ? node.children.map(textOf).join("") : "";
}

function diffLanguage(pre: Element): string | null {
  const [code] = pre.children;
  if (!code || code.type !== "element" || code.tagName !== "code") return null;
  const className = code.properties.className;
  const found = Array.isArray(className) ? className.find((c) => String(c).startsWith(LANGUAGE_CLASS)) : undefined;
  return found === undefined ? null : String(found).slice(LANGUAGE_CLASS.length);
}

/**
 * 各行の先頭の記号を外して言語の色分けを付け、記号を戻して、行に追加と削除のクラスを付ける。
 * 記号で始まらない行（@@ の行など）は、Zenn と同じく色を付けない。
 * 知らない言語なら、diff の色分けだけを付ける。
 */
export function highlightDiff(highlighter: HighlighterCore, options: DiffOptions, pre: Element): Element {
  const lang = diffLanguage(pre)!;
  const [code] = pre.children as Element[];
  if (!highlighter.getLoadedLanguages().includes(lang)) {
    code.properties.className = ["language-diff"];
    return pre;
  }

  const lines = textOf(code).replace(/\n$/, "").split("\n");
  const markers = lines.map((line) => (line[0] !== undefined && line[0] in MARKERS ? line[0] : null));
  const source = lines.map((line, i) => (markers[i] === null ? "" : line.slice(1))).join("\n");

  const fragment = highlighter.codeToHast(source, {
    ...options,
    lang,
    transformers: [
      {
        line(node, lineNumber) {
          const marker = markers[lineNumber - 1];
          if (marker === null) {
            node.children = [{ type: "text", value: lines[lineNumber - 1] } satisfies Text];
            return;
          }
          const className = MARKERS[marker];
          if (className) this.addClassToHast(node, className);
          node.children.unshift({
            type: "element",
            tagName: "span",
            properties: { className: ["diff-marker"] },
            children: [{ type: "text", value: marker }],
          });
        },
      },
    ],
  });
  return fragment.children[0] as Element;
}

/** rehype のプラグイン。`language-diff-<言語>` のコードブロックを色分けする。 */
export function rehypeHighlightDiff(highlighter: HighlighterCore, options: DiffOptions) {
  function visit(parent: Root | Element) {
    parent.children.forEach((child, index) => {
      if (child.type !== "element") return;
      if (child.tagName === "pre" && diffLanguage(child) !== null) {
        parent.children[index] = highlightDiff(highlighter, options, child);
        return;
      }
      visit(child);
    });
  }
  return (tree: Root) => visit(tree);
}
