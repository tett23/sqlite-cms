import { DIFF_LANGUAGE_PREFIX } from "./markdown/transforms";

/**
 * Shiki で色を付ける言語の名前と別名（highlight.ts の LANGUAGES と langAlias）。
 * Shiki を読み込む前に、色を付けるコードブロックがあるかを判断するために、本体に持つ（ADR 0029）。
 * highlight.ts に言語を足したら、ここにも足す（足し忘れるとテストが落ちる）。
 */
export const HIGHLIGHT_LANGUAGES: ReadonlySet<string> = new Set([
  "bash",
  "cjs",
  "css",
  "cts",
  "diff",
  "haskell",
  "hs",
  "html",
  "javascript",
  "js",
  "json",
  "jsx",
  "markdown",
  "md",
  "mjs",
  "mts",
  "py",
  "python",
  "rs",
  "rust",
  "sh",
  "shell",
  "shellscript",
  "sql",
  "toml",
  "ts",
  "tsx",
  "typescript",
  "yaml",
  "yml",
  "zsh",
]);

const LANGUAGE_PREFIX = "language-";

/**
 * コードブロックの code 要素のクラスから、Shiki で色を付けるかを判断する。
 * 言語名のないブロック、知らない言語（mermaid を含む）は色を付けないので、Shiki を読み込まない。
 * diff と言語を同時に指定したもの（language-diff-<言語>）は、diff の色分けができるので読み込む。
 */
export function needsHighlight(className: unknown): boolean {
  if (!Array.isArray(className)) return false;
  return className.some((name) => {
    if (typeof name !== "string" || !name.startsWith(LANGUAGE_PREFIX)) return false;
    const language = name.slice(LANGUAGE_PREFIX.length);
    return HIGHLIGHT_LANGUAGES.has(language) || language.startsWith(DIFF_LANGUAGE_PREFIX);
  });
}
