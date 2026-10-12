import { renderToStaticMarkup } from "preact-render-to-string";
import { beforeAll, describe, expect, it } from "vitest";
import { bundledLanguagesInfo } from "shiki/langs";
import { BACKGROUND, DIFF_BACKGROUNDS, getHighlighter, highlightBlockWhenReady, LANGUAGES, THEME_REGISTRATION } from "./highlight";
import { highlightLoader } from "./highlightLoader";
import { LANGUAGE_IDS } from "./highlightLanguageNames.generated";
import { needsHighlight } from "./highlightLanguages";
import { MarkdownBody } from "./MarkdownBody";

function render(source: string): string {
  return renderToStaticMarkup(<MarkdownBody source={source} />);
}

function fence(lang: string, code: string): string {
  return `\`\`\`${lang}\n${code}\n\`\`\``;
}

function colors(html: string): Set<string> {
  return new Set([...html.matchAll(/style="color:(#[0-9A-Fa-f]{3,8})/g)].map((m) => m[1].toLowerCase()));
}

function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, "")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&quot;", '"')
    .replaceAll("&#x27;", "'")
    .replaceAll("&amp;", "&");
}

const SAMPLES: Record<string, string> = {
  bat: "echo off\nset NAME=world\necho Hello %NAME%",
  c: "#include <stdio.h>\nint main(void) { return 0; }",
  clojure: "(defn add [a b] (+ a b))",
  cmake: "cmake_minimum_required(VERSION 3.20)\nproject(app)",
  csharp: "public class A { public int X = 1; }",
  css: "a { color: red; }",
  dart: "void main() { var x = 1; print(x); }",
  diff: "- old\n+ new",
  docker: "FROM node:24\nRUN npm ci",
  fish: "set -x NAME world\necho \"hello $NAME\"",
  fsharp: "let add a b = a + b",
  gleam: "pub fn main() { let x = 1 }",
  go: "package main\n\nfunc main() { x := 1 }",
  groovy: "def name = \"world\"\nprintln \"hello\"",
  haskell: "main :: IO ()\nmain = print (1 + 2)",
  hcl: "resource \"aws_instance\" \"a\" {\n  ami = \"x\"\n}",
  html: "<p class=\"note\">hi</p>",
  ini: "[core]\neditor = vim",
  java: "public class A { int x = 1; }",
  javascript: "const x = 1;",
  json: "{\"a\": 1}",
  jsonc: "{\n  // コメント\n  \"a\": 1\n}",
  jsx: "const A = () => <div className=\"x\" />;",
  kotlin: "fun main() { val x = 1 }",
  less: "@color: red;\na { color: @color; }",
  lua: "local x = 1\nprint(x)",
  make: "all: build\n\nbuild:\n\tcargo build",
  markdown: "# 見出し\n\n**強調**",
  nix: "{ pkgs }: pkgs.mkShell { name = \"x\"; }",
  ocaml: "let add a b = a + b",
  powershell: "$name = \"world\"\nWrite-Host \"hello\"",
  prisma: "model User {\n  id Int @id\n}",
  proto: "syntax = \"proto3\";\nmessage A { int32 x = 1; }",
  python: "def f(x):\n    return x + 1",
  r: "x <- c(1, 2)\nprint(x)",
  rust: "fn main() { let x = 1; }",
  sass: "$color: red\na\n  color: $color",
  scala: "object A { val x = 1 }",
  scss: "$color: red;\na { color: $color; }",
  shellscript: "echo \"$HOME\"",
  solidity: "contract A { uint x = 1; }",
  sql: "SELECT * FROM posts WHERE slug = 'a';",
  swift: "let x = 1\nprint(x)",
  terraform: "variable \"name\" {\n  default = \"x\"\n}",
  toml: "[deploy]\nworker = \"my-blog\"",
  tsx: "const A = () => <div className=\"x\" />;",
  typescript: "type T = { a: number };",
  xml: "<a href=\"x\">y</a>",
  yaml: "title: \"t\"",
  zig: "const x: u8 = 1;",
};

describe("シンタックスハイライト", () => {
  beforeAll(async () => {
    await highlightLoader.load();
  });

  it("登録した言語と見本の言語が一致する（言語を足したら見本も足す）", () => {
    // 中に含む言語（nix の markdown-nix など）は除き、各言語の本体の文法だけを比べる。
    const registered = new Set(LANGUAGES.map((grammars) => grammars[grammars.length - 1].name));
    expect([...registered].sort()).toEqual(Object.keys(SAMPLES).sort());
  });

  it.each(Object.entries(SAMPLES))("%s を複数の色に分ける", (lang, code) => {
    const html = render(fence(lang, code));
    expect(html).toContain('class="shiki github-light-high-contrast"');
    expect(colors(html).size).toBeGreaterThanOrEqual(2);
  });

  it.each(Object.entries(SAMPLES))("%s の色分けでコードの文字列は変わらない", (lang, code) => {
    expect(text(render(fence(lang, code)))).toBe(code);
  });

  it.each(["ts", "js", "sh", "bash", "zsh", "shell", "hs", "yml", "md", "py", "kt", "cs", "dockerfile", "tf"])(
    "別名 %s でも色を付ける",
    (alias) => {
      const html = render(fence(alias, "x = 1"));
      expect(html).toContain('class="shiki github-light-high-contrast"');
    },
  );

  it("テーマのすべての文字色が、背景色と diff の行の背景色に対して 4.5:1 以上のコントラストを持つ", () => {
    const channel = (c: number) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
    const luminance = (hex: string) => {
      const [r, g, b] = [1, 3, 5].map((i) => channel(parseInt(hex.slice(i, i + 2), 16) / 255));
      return 0.2126 * r + 0.7152 * g + 0.0722 * b;
    };
    const contrast = (a: string, b: string) => {
      const [light, dark] = [luminance(a), luminance(b)].sort((x, y) => y - x);
      return (light + 0.05) / (dark + 0.05);
    };
    const theme = THEME_REGISTRATION;
    const foregrounds = [
      theme.colors?.["editor.foreground"],
      ...(theme.tokenColors ?? []).filter((rule) => !rule.settings.background).map((rule) => rule.settings.foreground),
    ].filter((color): color is string => typeof color === "string");
    expect(foregrounds.length).toBeGreaterThan(10);
    for (const color of new Set(foregrounds.map((c) => c.slice(0, 7).toLowerCase()))) {
      for (const background of [BACKGROUND, ...DIFF_BACKGROUNDS]) {
        expect(contrast(color, background), `${color} / ${background}`).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  it("コードブロックの背景色を本文の pre と揃える", () => {
    expect(render(fence("rust", "fn main() {}"))).toContain("background-color:#f5f5f5");
  });

  it("登録していない言語と言語の指定がないブロックは色を付けない", () => {
    expect(render(fence("brainfuck", "+++."))).toBe(
      '<div class="article-body mt-6"><pre><code class="language-brainfuck">+++.\n</code></pre></div>',
    );
    expect(render("```\nplain\n```")).toBe('<div class="article-body mt-6"><pre><code>plain\n</code></pre></div>');
  });

  it("行内のコードには色を付けない", () => {
    expect(render("本文の `fn main()` です")).toContain("<code>fn main()</code>");
  });

  it("コードブロックの中の HTML は要素にせず文字として表示する", () => {
    const html = render(fence("html", "<script>alert(1)</script>"));
    expect(html).not.toContain("<script>");
    expect(text(html)).toBe("<script>alert(1)</script>");
  });

  it("ハイライタは一度だけ作って使い回す", () => {
    expect(getHighlighter()).toBe(getHighlighter());
  });
});

describe("使うときに読み込む言語（ADR 0067）", () => {
  beforeAll(async () => {
    await highlightLoader.load();
  });

  it("いつも読み込む言語にない言語も、文法を読み込んで色を付け、二度目は読み込み直さない", async () => {
    expect(getHighlighter().getLoadedLanguages()).not.toContain("ruby");
    const first = highlightBlockWhenReady('def hello(name) = puts "hi #{name}"', "language-ruby");
    expect(first).toBeInstanceOf(Promise);
    const block = await first;
    expect(block?.className).toBe("shiki github-light-high-contrast");
    expect(colors(block!.html).size).toBeGreaterThanOrEqual(2);
    // 別名でも、読み込み済みの文法で同期的に色を付ける。
    expect(highlightBlockWhenReady("puts 1", "language-rb")).not.toBeInstanceOf(Promise);
  });

  it("diff と言語の組では、言語のほうを読み込む", async () => {
    const block = await highlightBlockWhenReady("- <?php echo 1;\n+ <?php echo 2;", "language-diff-php");
    expect(block?.html).toContain("diff-add");
    expect(getHighlighter().getLoadedLanguages()).toContain("php");
  });

  it("いつも読み込む言語は、同期的に色を付ける", () => {
    expect(highlightBlockWhenReady("package main", "language-go")).not.toBeInstanceOf(Promise);
  });

  it("知らない言語は読み込まず、色を付けない", () => {
    expect(highlightBlockWhenReady("x", "language-brainfuck")).toBeNull();
  });
});

describe("Shiki を読み込む前の言語の判断（ADR 0029、0067）", () => {
  it("本体に持つ言語名の一覧が、Shiki の言語の id と別名に一致する（違えば npm run gen:languages で作り直す）", () => {
    const expected: Record<string, string> = {};
    for (const { id, aliases = [] } of bundledLanguagesInfo) {
      expected[id] = id;
      for (const alias of aliases) expected[alias] ??= id;
    }
    expect(LANGUAGE_IDS).toEqual(expected);
  });

  it("いつも読み込む言語は、すべて Shiki の言語にある", () => {
    for (const grammars of LANGUAGES) {
      const main = grammars[grammars.length - 1];
      expect(LANGUAGE_IDS[main.name], main.name).toBe(main.name);
    }
  });

  it("色を付ける言語と diff と言語の組み合わせだけを読み込みの対象にする", () => {
    expect(needsHighlight(["language-rust"])).toBe(true);
    expect(needsHighlight(["language-sh"])).toBe(true);
    expect(needsHighlight(["language-diff-ts"])).toBe(true);
    expect(needsHighlight(["language-ruby"])).toBe(true);
    expect(needsHighlight(["language-rb"])).toBe(true);
    expect(needsHighlight(["language-diff-ruby"])).toBe(true);
    expect(needsHighlight(["language-mermaid"])).toBe(false);
    expect(needsHighlight(["language-brainfuck"])).toBe(false);
    expect(needsHighlight(["language-constructor"])).toBe(false);
    expect(needsHighlight([])).toBe(false);
    expect(needsHighlight(undefined)).toBe(false);
    expect(needsHighlight(["rust"])).toBe(false);
  });
});
