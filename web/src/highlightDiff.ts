// diff と言語の色分けを同時に付ける（```diff js、ADR 0025）。Shiki の前に通す。
import type { Element, Text } from "hast";
import type { HighlighterCore } from "shiki/core";

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

/**
 * diff と言語を同時に指定したコードブロック（```diff js、ADR 0025）を色分けする。
 * 各行の先頭の記号を外して言語の色分けを付け、記号を戻して、行に追加と削除のクラスを付ける。
 * 記号で始まらない行（@@ の行など）は、Zenn と同じく色を付けない。
 * 知らない言語なら、diff の色分けだけを付ける。
 */
export function highlightDiff(highlighter: HighlighterCore, options: DiffOptions, lang: string, text: string): Element {
  if (!highlighter.getLoadedLanguages().includes(lang)) {
    return highlighter.codeToHast(text, { ...options, lang: "diff" }).children[0] as Element;
  }

  const lines = text.split("\n");
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
