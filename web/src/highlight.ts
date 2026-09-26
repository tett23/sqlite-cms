import css from "@shikijs/langs/css";
import diff from "@shikijs/langs/diff";
import haskell from "@shikijs/langs/haskell";
import html from "@shikijs/langs/html";
import javascript from "@shikijs/langs/javascript";
import json from "@shikijs/langs/json";
import markdown from "@shikijs/langs/markdown";
import python from "@shikijs/langs/python";
import rust from "@shikijs/langs/rust";
import shellscript from "@shikijs/langs/shellscript";
import sql from "@shikijs/langs/sql";
import toml from "@shikijs/langs/toml";
import tsx from "@shikijs/langs/tsx";
import typescript from "@shikijs/langs/typescript";
import yaml from "@shikijs/langs/yaml";
import githubLightHighContrast from "@shikijs/themes/github-light-high-contrast";
import type { Element } from "hast";
import { createHighlighterCore, hastToHtml, type HighlighterCore } from "shiki/core";
import { createOnigurumaEngine } from "shiki/engine/oniguruma";
import onigWasmUrl from "shiki/onig.wasm?url";
import { highlightDiff } from "./highlightDiff";
import { DIFF_LANGUAGE_PREFIX } from "./markdown/transforms";

// 文字色がすべて本文の pre の背景（#f5f5f5）に対して 4.5:1 以上になるテーマ（ADR 0019）。
export const THEME = "github-light-high-contrast";

// 言語を足すとバンドルが大きくなる（1 言語あたり gzip 後で 1〜16 KB）。
export const LANGUAGES = [
  css,
  diff,
  haskell,
  html,
  javascript,
  json,
  markdown,
  python,
  rust,
  shellscript,
  sql,
  toml,
  tsx,
  typescript,
  yaml,
];

// 本文の pre の背景色（index.css）とテーマの背景色を揃える。
export const BACKGROUND = "#f5f5f5";
// diff の追加と削除の行の背景色（index.css）。文字色はこれに対しても 4.5:1 以上にする。
export const DIFF_BACKGROUNDS = ["#e6ffec", "#fff0f0"];
export const THEME_REGISTRATION = githubLightHighContrast;
const COLOR_REPLACEMENTS = { "#ffffff": BACKGROUND };

let highlighter: HighlighterCore | null = null;
let creating: Promise<HighlighterCore> | null = null;

/**
 * 正規表現エンジン（Oniguruma）の wasm（ADR 0046）。
 * テスト（Node.js）では URL から取れないので、wasm を埋め込んだモジュールを使う。ビルドではこの分岐は消える。
 */
function onigWasm() {
  if (import.meta.env.MODE === "test") return import("shiki/wasm");
  return fetch(onigWasmUrl);
}

/**
 * ハイライタを一度だけ作る（ADR 0046）。
 * 正規表現は Oniguruma（wasm）で照合する。JavaScript の正規表現エンジンは、文法の正規表現を変換する手間と、
 * 変換した正規表現の照合（特に TypeScript の文法）が重く、最初のコードブロックの色分けが長いタスクになっていた。
 */
export function initHighlighter(): Promise<HighlighterCore> {
  creating ??= createHighlighterCore({
    themes: [githubLightHighContrast],
    langs: LANGUAGES,
    langAlias: { jsx: "tsx" },
    engine: createOnigurumaEngine(onigWasm()),
  }).then((created) => (highlighter = created));
  return creating;
}

/** 作ったハイライタ。initHighlighter が終わってから使う。 */
export function getHighlighter(): HighlighterCore {
  if (!highlighter) throw new Error("ハイライタを作る前に使いました（initHighlighter を待ってください）");
  return highlighter;
}

export const highlightOptions = { theme: THEME, colorReplacements: COLOR_REPLACEMENTS };

/** 色付けしたコードブロック。pre 要素のクラス、スタイル、中身の HTML。 */
export interface HighlightedBlock {
  className: string;
  style: Record<string, string>;
  html: string;
}

/** `background-color:#f5f5f5;color:#0e1116` を React の style にする。 */
function parseStyle(style: unknown): Record<string, string> {
  if (typeof style !== "string") return {};
  return Object.fromEntries(
    style
      .split(";")
      .map((declaration) => declaration.split(":").map((part) => part.trim()))
      .filter(([name, value]) => name && value)
      .map(([name, value]) => [name.replace(/-([a-z])/g, (_, c: string) => c.toUpperCase()), value]),
  );
}

/**
 * コードブロックを一つ色付けする（ADR 0038）。コードブロックの部品が、画面の近くに来たときに呼ぶ。
 * 本文の Markdown 全体を描き直さずに、そのブロックだけを色付けするためである。
 * languageClass は code 要素の `language-<言語名>`。色を付けられなければ null。
 */
export function highlightBlock(code: string, languageClass: string): HighlightedBlock | null {
  const text = code.endsWith("\n") ? code.slice(0, -1) : code;
  const lang = languageClass.replace(/^language-/, "");
  const highlighter = getHighlighter();
  let pre: Element;
  try {
    pre = lang.startsWith(DIFF_LANGUAGE_PREFIX)
      ? highlightDiff(highlighter, highlightOptions, lang.slice(DIFF_LANGUAGE_PREFIX.length), text)
      : (highlighter.codeToHast(text, { ...highlightOptions, lang }).children[0] as Element);
  } catch {
    return null;
  }
  // Shiki の hast は、クラスを class（文字列）で持つ。
  const className = pre.properties.class ?? pre.properties.className;
  return {
    className: Array.isArray(className) ? className.join(" ") : String(className ?? ""),
    style: parseStyle(pre.properties.style),
    // 文字参照は短い書き方（&lt; など）にする。
    html: hastToHtml({ type: "root", children: pre.children }, { characterReferences: { useShortestReferences: true } }),
  };
}
