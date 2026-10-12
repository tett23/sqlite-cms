import bat from "@shikijs/langs/bat";
import cLang from "@shikijs/langs/c";
import clojure from "@shikijs/langs/clojure";
import cmake from "@shikijs/langs/cmake";
import csharp from "@shikijs/langs/csharp";
import css from "@shikijs/langs/css";
import dart from "@shikijs/langs/dart";
import diff from "@shikijs/langs/diff";
import docker from "@shikijs/langs/docker";
import fish from "@shikijs/langs/fish";
import fsharp from "@shikijs/langs/fsharp";
import gleam from "@shikijs/langs/gleam";
import goLang from "@shikijs/langs/go";
import groovy from "@shikijs/langs/groovy";
import haskell from "@shikijs/langs/haskell";
import hcl from "@shikijs/langs/hcl";
import html from "@shikijs/langs/html";
import ini from "@shikijs/langs/ini";
import java from "@shikijs/langs/java";
import javascript from "@shikijs/langs/javascript";
import json from "@shikijs/langs/json";
import jsonc from "@shikijs/langs/jsonc";
import jsx from "@shikijs/langs/jsx";
import kotlin from "@shikijs/langs/kotlin";
import less from "@shikijs/langs/less";
import lua from "@shikijs/langs/lua";
import makefile from "@shikijs/langs/make";
import markdown from "@shikijs/langs/markdown";
import nix from "@shikijs/langs/nix";
import ocaml from "@shikijs/langs/ocaml";
import powershell from "@shikijs/langs/powershell";
import prisma from "@shikijs/langs/prisma";
import proto from "@shikijs/langs/proto";
import python from "@shikijs/langs/python";
import rLang from "@shikijs/langs/r";
import rust from "@shikijs/langs/rust";
import sass from "@shikijs/langs/sass";
import scala from "@shikijs/langs/scala";
import scss from "@shikijs/langs/scss";
import shellscript from "@shikijs/langs/shellscript";
import solidity from "@shikijs/langs/solidity";
import sql from "@shikijs/langs/sql";
import swift from "@shikijs/langs/swift";
import terraform from "@shikijs/langs/terraform";
import toml from "@shikijs/langs/toml";
import tsx from "@shikijs/langs/tsx";
import typescript from "@shikijs/langs/typescript";
import xml from "@shikijs/langs/xml";
import yaml from "@shikijs/langs/yaml";
import zig from "@shikijs/langs/zig";
import githubLightHighContrast from "@shikijs/themes/github-light-high-contrast";
import type { Element } from "hast";
import { createHighlighterCore, hastToHtml, type HighlighterCore } from "shiki/core";
import { bundledLanguages } from "shiki/langs";
import { createOnigurumaEngine } from "shiki/engine/oniguruma";
import onigWasmUrl from "shiki/onig.wasm?url";
import { highlightDiff } from "./highlightDiff";
import { LANGUAGE_IDS } from "./highlightLanguageNames.generated";
import { DIFF_LANGUAGE_PREFIX } from "./markdown/transforms";

// 文字色がすべて本文の pre の背景（#f5f5f5）に対して 4.5:1 以上になるテーマ（ADR 0019）。
export const THEME = "github-light-high-contrast";

// いつも読み込む言語（ADR 0067）。Shiki と同じチャンクに入る。文法が小さく、よく書かれるものを選んだ。
// ここに足すとチャンクが大きくなる（1 言語あたり gzip 後で 1〜16 KB）。ほかの言語は、使うときに読み込む（loadLanguage）。
export const LANGUAGES = [
  bat,
  cLang,
  clojure,
  cmake,
  csharp,
  css,
  dart,
  diff,
  docker,
  fish,
  fsharp,
  gleam,
  goLang,
  groovy,
  haskell,
  hcl,
  html,
  ini,
  java,
  javascript,
  json,
  jsonc,
  jsx,
  kotlin,
  less,
  lua,
  makefile,
  markdown,
  nix,
  ocaml,
  powershell,
  prisma,
  proto,
  python,
  rLang,
  rust,
  sass,
  scala,
  scss,
  shellscript,
  solidity,
  sql,
  swift,
  terraform,
  toml,
  tsx,
  typescript,
  xml,
  yaml,
  zig,
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

/** 読み込んでいる途中か、読み込み終えた言語（id ごと）。同じ言語を二度読み込まない。 */
const loadingLanguages = new Map<string, Promise<void>>();

/**
 * いつも読み込む言語にない言語の文法を、使うときに読み込む（ADR 0067）。文法は言語ごとに別のチャンクになっている。
 * 中に別の言語を含む文法は、Shiki の言語のモジュールが依存ごと返す。読み込めなくても失敗にはせず、色なしのままにする。
 * 読み込む必要があれば Promise を、なければ null を返す。
 */
export function loadLanguage(name: string): Promise<void> | null {
  const id = Object.hasOwn(LANGUAGE_IDS, name) ? LANGUAGE_IDS[name] : undefined;
  if (id === undefined || getHighlighter().getLoadedLanguages().includes(id)) return null;
  let loading = loadingLanguages.get(id);
  if (!loading) {
    const grammar = bundledLanguages[id as keyof typeof bundledLanguages];
    loading = grammar
      ? getHighlighter()
          .loadLanguage(grammar)
          .catch((error: unknown) => console.error(`言語 ${id} の文法を読み込めませんでした`, error))
      : Promise.resolve();
    loadingLanguages.set(id, loading);
  }
  return loading;
}

/**
 * 言語の文法を読み込んでから、コードブロックを色付けする（ADR 0067）。
 * いつも読み込む言語と、読み込み済みの言語なら、同期的に色付けする。diff と言語の組（`diff-<言語>`）は、言語のほうを読み込む。
 */
export function highlightBlockWhenReady(code: string, languageClass: string): HighlightedBlock | null | Promise<HighlightedBlock | null> {
  const lang = languageClass.replace(/^language-/, "");
  const loading = loadLanguage(lang.startsWith(DIFF_LANGUAGE_PREFIX) ? lang.slice(DIFF_LANGUAGE_PREFIX.length) : lang);
  return loading ? loading.then(() => highlightBlock(code, languageClass)) : highlightBlock(code, languageClass);
}
