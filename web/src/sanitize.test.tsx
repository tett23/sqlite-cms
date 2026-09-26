import { renderToStaticMarkup } from "react-dom/server";
import { beforeAll, describe, expect, it } from "vitest";
import { highlightLoader } from "./highlightLoader";
import { MarkdownBody } from "./MarkdownBody";

function render(source: string): string {
  return renderToStaticMarkup(<MarkdownBody source={source} />);
}

describe("本文の HTML", () => {
  it("details の中の Markdown は、空行で区切れば処理される", () => {
    const html = render("<details>\n<summary>補足</summary>\n\n中身は **Markdown**。\n\n</details>");
    expect(html).toContain("<details>");
    expect(html).toContain("<summary>補足</summary>");
    expect(html).toContain("<p>中身は <strong>Markdown</strong>。</p>");
  });

  it("details の open と、summary の中の strong と code を残す", () => {
    const html = render("<details open>\n<summary><strong>補足</strong>と<code>x</code></summary>\n\n本文\n\n</details>");
    expect(html).toContain('<details open="">');
    expect(html).toContain("<summary><strong>補足</strong>と<code>x</code></summary>");
  });

  it.each([
    ["kbd", "<kbd>Ctrl</kbd> + <kbd>C</kbd>", "<kbd>Ctrl</kbd>"],
    ["sub", "H<sub>2</sub>O", "<sub>2</sub>"],
    ["sup", "x<sup>2</sup>", "<sup>2</sup>"],
    ["ruby", "<ruby>漢字<rp>(</rp><rt>かんじ</rt><rp>)</rp></ruby>", "<ruby>漢字<rp>(</rp><rt>かんじ</rt><rp>)</rp></ruby>"],
    ["mark", "<mark>強調</mark>", "<mark>強調</mark>"],
    ["abbr", '<abbr title="HyperText Markup Language">HTML</abbr>', '<abbr title="HyperText Markup Language">HTML</abbr>'],
    ["dl", "<dl><dt>用語</dt><dd>説明</dd></dl>", "<dl><dt>用語</dt><dd>説明</dd></dl>"],
    ["表の中の br", "| a |\n|---|\n| 1 行目<br>2 行目 |", "<td>1 行目<br/>2 行目</td>"],
  ])("%s を残す", (_name, source, expected) => {
    expect(render(source)).toContain(expected);
  });

  it("許可しない要素はタグだけを外し、中身の文字は残す", () => {
    const html = render("<div>div</div>\n\n<span>span</span> <b>b</b> <i>i</i> <u>u</u> <s>s</s>");
    for (const tag of ["div", "span", "b", "i", "u", "s"]) {
      expect(html).not.toContain(`<${tag}>`);
      expect(html).toContain(tag);
    }
  });

  it("script、iframe、style、object は中身ごと取り除く", () => {
    const html = render(
      '<script>alert(1)</script>\n\n<iframe src="https://example.com"></iframe>\n\n<style>body{}</style>\n\n<object data="x"></object>\n\n本文',
    );
    expect(html).not.toMatch(/<(script|iframe|style|object)/);
    expect(html).not.toContain("alert(1)");
    expect(html).toContain("本文");
  });

  it("イベント属性、style、class を取り除く", () => {
    const html = render('<kbd onclick="alert(1)" style="color:red" class="x">K</kbd>');
    expect(html).toContain("<kbd>K</kbd>");
  });

  it("リンク先は http、https、mailto、サイト内だけを残す", () => {
    const html = render(
      '[a](https://example.com) [b](mailto:a@example.com) <a href="/about">c</a> [d](javascript:alert(1)) <a href="data:text/html,x">e</a>',
    );
    expect(html).toContain('href="https://example.com"');
    expect(html).toContain('href="mailto:a@example.com"');
    expect(html).toContain('href="/about"');
    expect(html).not.toContain("javascript:");
    expect(html).not.toContain("data:text");
  });

  it("本文に書いた id は取り除き、脚注のリンクはすべて行き先に届く", () => {
    const html = render('本文[^1]と[^note]\n\n[^1]: 注。\n[^note]: 二つ目。\n\n<h2 id="root">見出し</h2>');
    expect(html).not.toContain('id="root"');
    const ids = new Set([...html.matchAll(/id="([^"]+)"/g)].map((m) => m[1]));
    const targets = [
      ...[...html.matchAll(/href="#([^"]+)"/g)].map((m) => m[1]),
      ...[...html.matchAll(/aria-describedby="([^"]+)"/g)].map((m) => m[1]),
    ];
    expect(targets.length).toBeGreaterThan(0);
    for (const target of targets) {
      expect(ids).toContain(target);
    }
    expect(html).not.toContain("user-content-user-content");
  });

  it("タスクリストのチェックボックスを残し、項目の文章を読み上げ用の名前にする", () => {
    const html = render("- [x] 済んだこと\n- [ ] まだのこと\n  - [ ] 入れ子");
    expect(html).toMatch(/<input type="checkbox" disabled="" aria-label="済んだこと" checked=""\/>/);
    expect(html).toContain('aria-label="まだのこと"');
    expect(html).toContain('aria-label="入れ子"');
  });

  it("タスクリストでない項目のチェックボックスには名前を付けない", () => {
    expect(render("- 普通の項目")).not.toContain("aria-label");
  });

  it("脚注の戻りリンクの読み上げ用の名前に、見えている記号を含める", () => {
    const html = render("本文[^1]\n\n[^1]: 注。");
    expect(html).toMatch(/aria-label="↩︎ 本文に戻る"[^>]*>↩︎<\/a>/);
  });

  it("init が作るトップページのコメントは表示しない", () => {
    const html = render("<!-- トップページの本文をここに Markdown で書く。このコメントは表示されない -->\n");
    expect(html).toBe('<div class="article-body mt-6"></div>');
  });

  describe("シンタックスハイライトとの組み合わせ", () => {
    beforeAll(async () => {
      await highlightLoader.load();
    });

    it("sanitize の後でも Shiki の色分けが残る", () => {
      const html = render("```rust\nfn main() {}\n```");
      expect(html).toContain('class="shiki github-light-high-contrast"');
      expect(html).toMatch(/style="color:#/);
    });

    it("details の中のコードブロックにも色が付く", () => {
      const html = render("<details>\n<summary>コード</summary>\n\n```rust\nfn main() {}\n```\n\n</details>");
      expect(html).toContain("<details>");
      expect(html).toContain('class="shiki github-light-high-contrast"');
    });
  });
});
