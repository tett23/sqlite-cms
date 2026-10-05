import { renderToStaticMarkup } from "preact-render-to-string";
import { beforeAll, describe, expect, it } from "vitest";
import { highlightLoader } from "../highlightLoader";
import { MarkdownBody } from "../MarkdownBody";
import { katexLoader, linkCardLoader } from "../render/loaders";
import { parseContainerInfo } from "./container";
import { parseCodeInfo } from "./transforms";

function render(source: string): string {
  return renderToStaticMarkup(<MarkdownBody source={source} />).replace(/^<div class="article-body mt-6">|<\/div>$/g, "");
}

describe(":::message と :::details", () => {
  it("メッセージを枠にする", () => {
    expect(render(":::message\nメッセージ **強調**\n:::")).toBe(
      '<div role="note" class="message"><p>メッセージ <strong>強調</strong></p></div>',
    );
  });

  it("alert を付けると警告の枠にする", () => {
    expect(render(":::message alert\n警告\n:::")).toBe(
      '<div role="note" class="message message-alert"><p>警告</p></div>',
    );
  });

  it("details は見出し付きの折りたたみにする。見出しを省くと「詳細」", () => {
    expect(render(":::details タイトル\n中身\n:::")).toBe("<details><summary>タイトル</summary><p>中身</p></details>");
    expect(render(":::details\n中身\n:::")).toBe("<details><summary>詳細</summary><p>中身</p></details>");
  });

  it("中身の Markdown（リスト、コードブロック、脚注）を処理する", () => {
    const html = render(":::message\n- a\n- b\n\n```js\nconst a = 1;\n```\n:::");
    expect(html).toContain("<ul>\n<li>a</li>\n<li>b</li>\n</ul>");
    expect(html).toContain('<pre><code class="language-js">const a = 1;\n</code></pre>');
  });

  it("外側のコロンを増やすと入れ子にできる", () => {
    expect(render("::::details 外\n:::message\n内\n:::\n\n後\n::::")).toBe(
      '<details><summary>外</summary><div role="note" class="message"><p>内</p></div><p>後</p></details>',
    );
  });

  it("同じ長さの ::: は外側を閉じる（Zenn と同じ）。内側は閉じのないまま外側の中身の終わりまで続く", () => {
    expect(render(":::details 外\n:::message\n内\n:::\n:::")).toBe(
      '<details><summary>外</summary><div role="note" class="message"><p>内</p></div></details>\n<p>:::</p>',
    );
  });

  it("リストの項目の中にも書ける", () => {
    expect(render("- 項目\n  :::message\n  中\n  :::\n- 次")).toContain(
      '<li>項目\n<div role="note" class="message"><p>中</p></div>\n</li>',
    );
  });

  it("段落の直後にも書ける", () => {
    expect(render("段落\n:::message\n中\n:::")).toBe('<p>段落</p>\n<div role="note" class="message"><p>中</p></div>');
  });

  it("閉じがなければ文書の終わりまでを中身にする", () => {
    expect(render(":::message\n中\n\n続き")).toBe('<div role="note" class="message"><p>中</p><p>続き</p></div>');
  });

  it("知らない名前やコロンが足りないものは段落のまま", () => {
    expect(render(":::foo\n中\n:::")).toBe("<p>:::foo\n中\n:::</p>");
    expect(render("::message\n中\n::")).toBe("<p>::message\n中\n::</p>");
    expect(render(":::message info\n中\n:::")).toBe("<p>:::message info\n中\n:::</p>");
  });

  it("開きの行を読む", () => {
    expect(parseContainerInfo("message")).toEqual({ kind: "message", alert: false, title: "" });
    expect(parseContainerInfo("message alert")).toEqual({ kind: "message", alert: true, title: "" });
    expect(parseContainerInfo("details  見出し  ")).toEqual({ kind: "details", alert: false, title: "見出し" });
    expect(parseContainerInfo("unknown")).toBeNull();
  });
});

describe("数式", () => {
  it("$$ で囲んだブロックと $ で囲んだ数式を取り出す（KaTeX を読み込む前は TeX のまま）", () => {
    expect(render("$$\ne^{i\\theta} = \\cos\\theta\n$$")).toBe('<div class="math-display">e^{i\\theta} = \\cos\\theta</div>');
    expect(render("式 $a\\ne0$ です")).toBe('<p>式 <span class="math-inline">a\\ne0</span> です</p>');
  });

  it("金額などの $ は数式にしない", () => {
    expect(render("$5 と $10")).toBe("<p>$5 と $10</p>");
    expect(render("$ a $")).toBe("<p>$ a $</p>");
    expect(render("$a $b")).toBe("<p>$a $b</p>");
    expect(render("\\$a$")).toBe("<p>$a$</p>");
  });

  it("\\$ は閉じにならず、コードの中の $ は数式にしない", () => {
    expect(render("$a\\$b$")).toBe('<p><span class="math-inline">a\\$b</span></p>');
    expect(render("`$a$`")).toBe("<p><code>$a$</code></p>");
  });

  it("$$ の行に式を書いたものはブロックにしない", () => {
    expect(render("$$x$$")).not.toContain("math-display");
  });

  describe("KaTeX を読み込んだ後", () => {
    beforeAll(async () => {
      await katexLoader.load();
    });

    it("数式を描画し、読み上げ用の MathML を付ける", () => {
      const html = render("$$\nx^2\n$$\n\n式 $a$ です");
      expect(html).toMatch(/<div class="math-display"><span class="katex-display"><span class="katex"><span class="katex-mathml"><math/);
      expect(html).toMatch(/<span class="math-inline"><span class="katex"><span class="katex-mathml"><math/);
    });

    it("書き誤りは TeX の文字列と日本語の説明を表示する", () => {
      expect(render("式 $\\frac{a$ です")).toContain(
        '<span class="math-inline math-error"><code>\\frac{a</code><span class="math-error-message">（数式を解釈できませんでした）</span></span>',
      );
    });
  });
});

describe("コードブロックの情報文字列", () => {
  it("言語、ファイル名、diff を読む", () => {
    expect(parseCodeInfo("js:foo.js", null)).toEqual({ language: "js", filename: "foo.js", diff: false, meta: "" });
    expect(parseCodeInfo(":foo.txt", null)).toEqual({ language: "", filename: "foo.txt", diff: false, meta: "" });
    expect(parseCodeInfo("diff", "js:foo.js")).toEqual({ language: "js", filename: "foo.js", diff: true, meta: "" });
    expect(parseCodeInfo("diff", null)).toEqual({ language: "diff", filename: "", diff: false, meta: "" });
    expect(parseCodeInfo("rust", null)).toEqual({ language: "rust", filename: "", diff: false, meta: "" });
  });

  it("ファイル名を figure と figcaption で表示する", () => {
    expect(render("```js:fooBar.js\nconst a = 1;\n```")).toBe(
      '<figure><figcaption>fooBar.js</figcaption><pre><code class="language-js">const a = 1;\n</code></pre></figure>',
    );
    expect(render("```:memo.txt\nメモ\n```")).toBe(
      "<figure><figcaption>memo.txt</figcaption><pre><code>メモ\n</code></pre></figure>",
    );
  });

  describe("Shiki を読み込んだ後", () => {
    beforeAll(async () => {
      await highlightLoader.load();
    });

    it("diff と言語を指定すると、行に追加と削除のクラスと記号を付けて言語の色分けをする", () => {
      const html = render("```diff js\n@@ -1 +1 @@\n+const a = 1;\n-let a = 1;\n unchanged();\n```");
      expect(html).toContain('<span class="line">@@ -1 +1 @@</span>');
      expect(html).toMatch(/<span class="line diff-add"><span class="diff-marker">\+<\/span><span style="color:#[0-9A-F]{6}">const<\/span>/);
      expect(html).toMatch(/<span class="line diff-remove"><span class="diff-marker">-<\/span><span style="color:#[0-9A-F]{6}">let<\/span>/);
      expect(html).toMatch(/<span class="line"><span class="diff-marker"> <\/span>/);
    });

    it("知らない言語なら diff の色分けだけを付ける", () => {
      const html = render("```diff brainfuck\n+++.\n```");
      expect(html).toContain('class="shiki github-light-high-contrast"');
      expect(html).not.toContain("diff-add");
    });

    it("mermaid のコードブロックには色を付けない", () => {
      expect(render("```mermaid\ngraph TB\n  A --> B\n```")).toBe(
        '<pre><code class="language-mermaid">graph TB\n  A --> B</code></pre>',
      );
    });
  });
});

describe("インラインの脚注", () => {
  it("^[内容] を脚注にし、GFM の脚注と参照の順に番号を振る", () => {
    const html = render("前[^a]と^[インライン **強調**]の後。\n\n[^a]: 通常");
    expect(html).toMatch(/前<sup><a href="#user-content-fn-a"[^>]*>1<\/a><\/sup>と<sup><a href="#user-content-fn-inline-1"[^>]*>2<\/a><\/sup>の後。/);
    expect(html).toContain('<li id="user-content-fn-inline-1">\n<p>インライン <strong>強調</strong> <a href="#user-content-fnref-inline-1"');
  });

  it("中の [ と ] の対応を数え、リンクを含められる", () => {
    const html = render("本文^[[括弧] と [リンク](https://example.com)]。");
    expect(html).toContain('<p>[括弧] と <a href="https://example.com">リンク</a> <a href="#user-content-fnref-inline-1"');
    expect(html).toContain("本文<sup>");
    expect(html).toContain("</sup>。</p>");
  });

  it("書き手の脚注の名前と重ならない名前を付ける", () => {
    const html = render("a[^inline-1] b^[自動]\n\n[^inline-1]: 手書き");
    expect(html).toContain('href="#user-content-fn-inline-2"');
  });

  it("閉じのないものは文字のまま", () => {
    expect(render("^[閉じない")).toBe("<p>^[閉じない</p>");
  });
});

describe("画像の幅", () => {
  it("=250x で幅を指定する（サイト内と外部の URL）", () => {
    expect(render("![図](/media/a.png =250x)")).toContain('<img src="/media/a.png" alt="図" width="250"/>');
    expect(render("![](https://example.com/a.png =120x) 後")).toContain(
      '<p><img src="https://example.com/a.png" alt width="120"/> 後</p>',
    );
  });

  it("リンクの中の画像にも使える", () => {
    expect(render("[![](/a.png =100x)](https://example.com)")).toContain(
      '<a href="https://example.com"><img src="/a.png" alt width="100"/></a>',
    );
  });

  it("形の違うものは文字のまま", () => {
    expect(render("![](/a.png =wide)")).toBe("<p>![](/a.png =wide)</p>");
  });

  it("画像の直後の行の強調は、そのまま em で出す（CSS で説明として表示する）", () => {
    expect(render("![図](/a.png)\n*説明*")).toContain('<img src="/a.png" alt="図"/>\n<em>説明</em>');
  });
});

describe("リンクカード", () => {
  // カードの部品は後から読み込む（ADR 0049）。読み込む前の表示は linkCard.test.tsx で確かめる。
  beforeAll(async () => {
    await linkCardLoader.load();
  });

  it("URL だけの段落をカードにする", () => {
    expect(render("https://example.com/path")).toBe(
      '<a href="https://example.com/path" class="link-card"><span class="link-card-text"><span class="link-card-host">example.com</span><span class="link-card-url">https://example.com/path</span></span></a>',
    );
    expect(render("<https://example.com>")).toContain('class="link-card"');
  });

  it("ビルドのときに取得した画像があれば、カードに表示する（代替テキストは空）", () => {
    const images = new Map([["https://example.com/path", "/link-cards/0123456789abcdef.webp"]]);
    const html = renderToStaticMarkup(<MarkdownBody source={"https://example.com/path\n\nhttps://example.com/other"} linkCardImages={images} />);
    expect(html).toContain(
      '<span class="link-card-url">https://example.com/path</span></span><img src="/link-cards/0123456789abcdef.webp" alt loading="lazy" class="link-card-image"/></a>',
    );
    // 画像のない URL のカードには img を付けない。
    expect(html.match(/<img /g)).toHaveLength(1);
  });

  it("本文に HTML で書いた img の class と loading は、カードのものだけを残す", () => {
    const html = render('<img src="/a.png" alt="a" class="evil" loading="eager"> <img src="/b.png" alt="b" class="link-card-image" loading="lazy">');
    expect(html).toContain('<img src="/a.png" alt="a"/>');
    expect(html).toContain('<img src="/b.png" alt="b" loading="lazy" class="link-card-image"/>');
  });

  it("文中の URL、リンク文字列のあるリンク、www. で始まるものはカードにしない", () => {
    expect(render("文中の https://example.com です")).not.toContain("link-card");
    expect(render("[例](https://example.com)")).not.toContain("link-card");
    expect(render("www.example.com")).not.toContain("link-card");
  });
});
