import { LANGUAGE_IDS } from "./highlightLanguageNames.generated";
import { DIFF_LANGUAGE_PREFIX } from "./markdown/transforms";

/**
 * 名前か別名に当たる Shiki の言語の id（ADR 0067）。いつも読み込む言語と、使うときに読み込む言語の両方を含む。
 * mermaid は図として描くので（ADR 0025）、色を付ける言語に含めない。
 */
export function languageId(name: string): string | undefined {
  const id = Object.hasOwn(LANGUAGE_IDS, name) ? LANGUAGE_IDS[name] : undefined;
  return id === "mermaid" ? undefined : id;
}

const LANGUAGE_PREFIX = "language-";

/**
 * コードブロックの code 要素のクラスから、Shiki で色を付けるかを判断する（ADR 0029、0067）。
 * 言語名のないブロック、知らない言語、mermaid は色を付けないので、Shiki を読み込まない。
 * diff と言語を同時に指定したもの（language-diff-<言語>）は、diff の色分けができるので読み込む。
 */
export function needsHighlight(className: unknown): boolean {
  if (!Array.isArray(className)) return false;
  return className.some((name) => {
    if (typeof name !== "string" || !name.startsWith(LANGUAGE_PREFIX)) return false;
    const language = name.slice(LANGUAGE_PREFIX.length);
    return languageId(language) !== undefined || language.startsWith(DIFF_LANGUAGE_PREFIX);
  });
}

/** ファイル名そのものから決める言語（大文字と小文字を区別する）。 */
export const FILE_NAME_LANGUAGES: Readonly<Record<string, string>> = {
  Dockerfile: "docker",
  Containerfile: "docker",
  Makefile: "make",
  GNUmakefile: "make",
  makefile: "make",
  "CMakeLists.txt": "cmake",
  Gemfile: "ruby",
  Rakefile: "ruby",
  Podfile: "ruby",
  Brewfile: "ruby",
  Vagrantfile: "ruby",
  Justfile: "just",
  justfile: "just",
  ".bashrc": "shellscript",
  ".bash_profile": "shellscript",
  ".zshrc": "shellscript",
  ".zprofile": "shellscript",
  ".profile": "shellscript",
  ".vimrc": "viml",
  ".gitconfig": "ini",
  ".editorconfig": "ini",
  ".npmrc": "ini",
  "flake.lock": "json",
  "go.mod": "go",
  "Cargo.lock": "toml",
};

/** Shiki の言語の名前と別名にない拡張子（小文字）から決める言語。 */
export const EXTENSION_LANGUAGES: Readonly<Record<string, string>> = {
  h: "c",
  hh: "cpp",
  hpp: "cpp",
  cc: "cpp",
  cxx: "cpp",
  cs: "csharp",
  ex: "elixir",
  exs: "elixir",
  erl: "erlang",
  hrl: "erlang",
  tf: "terraform",
  tfvars: "terraform",
  htm: "html",
  xhtml: "html",
  markdown: "markdown",
  mdx: "mdx",
  kts: "kotlin",
  mli: "ocaml",
  fsi: "fsharp",
  fsx: "fsharp",
  clj: "clojure",
  cljs: "clojure",
  cljc: "clojure",
  edn: "clojure",
  pl: "perl",
  pm: "perl",
  ps1: "powershell",
  psm1: "powershell",
  cmd: "bat",
  mk: "make",
  svg: "xml",
  plist: "xml",
  xsd: "xml",
  cfg: "ini",
  conf: "ini",
  gql: "graphql",
  m: "objective-c",
  mm: "objective-cpp",
  sol: "solidity",
  gradle: "groovy",
  rake: "ruby",
  gemspec: "ruby",
  sty: "latex",
  cls: "latex",
};

/**
 * ファイル名から、Shiki で色を付ける言語の id を決める（コードの埋め込み、ADR 0065、0067）。
 * ファイル名の表、拡張子が言語の名前か別名に当たるもの、拡張子の表の順に見る。どれにも当たらなければ undefined（色を付けない）。
 */
export function languageOfFile(fileName: string): string | undefined {
  if (Object.hasOwn(FILE_NAME_LANGUAGES, fileName)) return languageId(FILE_NAME_LANGUAGES[fileName]);
  const dot = fileName.lastIndexOf(".");
  if (dot <= 0) return undefined;
  const extension = fileName.slice(dot + 1).toLowerCase();
  return languageId(extension) ?? (Object.hasOwn(EXTENSION_LANGUAGES, extension) ? languageId(EXTENSION_LANGUAGES[extension]) : undefined);
}
