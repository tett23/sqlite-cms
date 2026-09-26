import { renderToStaticMarkup } from "react-dom/server";
import { beforeAll, describe, expect, it } from "vitest";
import { getHighlighter, LANGUAGES } from "./highlight";
import { highlightLoader } from "./highlightLoader";
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
  css: "a { color: red; }",
  diff: "- old\n+ new",
  haskell: "main :: IO ()\nmain = print (1 + 2)",
  html: '<p class="note">hi</p>',
  javascript: "const x = 1;",
  json: '{"a": 1}',
  markdown: "# 見出し\n\n**強調**",
  python: "def f(x):\n    return x + 1",
  rust: "fn main() { let x = 1; }",
  shellscript: 'echo "$HOME"',
  sql: "SELECT * FROM posts WHERE slug = 'a';",
  toml: '[deploy]\nworker = "my-blog"',
  tsx: "const A = () => <div className=\"x\" />;",
  typescript: "type T = { a: number };",
  yaml: 'title: "t"',
};

describe("シンタックスハイライト", () => {
  beforeAll(async () => {
    await highlightLoader.load();
  });

  it("登録した言語と見本の言語が一致する（言語を足したら見本も足す）", () => {
    const registered = new Set(LANGUAGES.flat().map((lang) => lang.name));
    expect([...registered].sort()).toEqual(Object.keys(SAMPLES).sort());
  });

  it.each(Object.entries(SAMPLES))("%s を複数の色に分ける", (lang, code) => {
    const html = render(fence(lang, code));
    expect(html).toContain('class="shiki github-light"');
    expect(colors(html).size).toBeGreaterThanOrEqual(2);
  });

  it.each(Object.entries(SAMPLES))("%s の色分けでコードの文字列は変わらない", (lang, code) => {
    expect(text(render(fence(lang, code)))).toBe(code);
  });

  it.each(["ts", "js", "jsx", "sh", "bash", "zsh", "shell", "hs", "yml", "md", "py"])(
    "別名 %s でも色を付ける",
    (alias) => {
      const html = render(fence(alias, "x = 1"));
      expect(html).toContain('class="shiki github-light"');
    },
  );

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
