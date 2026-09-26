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
import rehypeShikiFromHighlighter from "@shikijs/rehype/core";
import type { Options } from "react-markdown";
import { createHighlighterCoreSync, type HighlighterCore } from "shiki/core";
import { createJavaScriptRegexEngine } from "shiki/engine/javascript";

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
export const THEME_REGISTRATION = githubLightHighContrast;
const COLOR_REPLACEMENTS = { "#ffffff": BACKGROUND };

let highlighter: HighlighterCore | null = null;

export function getHighlighter(): HighlighterCore {
  highlighter ??= createHighlighterCoreSync({
    themes: [githubLightHighContrast],
    langs: LANGUAGES,
    langAlias: { jsx: "tsx" },
    engine: createJavaScriptRegexEngine(),
  });
  return highlighter;
}

export const highlightOptions = { theme: THEME, colorReplacements: COLOR_REPLACEMENTS };

export function createRehypePlugins(): Options["rehypePlugins"] {
  return [[rehypeShikiFromHighlighter, getHighlighter(), highlightOptions]];
}
