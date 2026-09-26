import { renderToStaticMarkup } from "preact-render-to-string";
import { describe, expect, it } from "vitest";
import { MarkdownBody } from "./MarkdownBody";

function render(source: string): string {
  return renderToStaticMarkup(<MarkdownBody source={source} />);
}

describe("MarkdownBody", () => {
  it("CommonMark の基本記法を描画する", () => {
    const html = render("## 見出し\n\n本文の**段落**。");
    expect(html).toContain("<h2>見出し</h2>");
    expect(html).toContain("<strong>段落</strong>");
  });

  it("GFM の表を描画する", () => {
    const html = render("| a | b |\n|---|---|\n| 1 | 2 |");
    expect(html).toContain("<table>");
    expect(html).toContain("<td>1</td>");
  });

  it("GFM の取り消し線を描画する", () => {
    expect(render("~~消す~~")).toContain("<del>消す</del>");
  });

  it("GFM のタスクリストを描画する", () => {
    const html = render("- [x] 済\n- [ ] 未");
    expect(html).toContain('class="contains-task-list"');
    expect(html).toMatch(/<input type="checkbox" checked disabled[^>]*\/>/);
  });

  it("GFM の自動リンクを描画する", () => {
    expect(render("www.example.com")).toContain('<a href="http://www.example.com">');
  });

  it("GFM の脚注を日本語のラベルで描画する", () => {
    const html = render("本文[^1]\n\n[^1]: 注。");
    expect(html).toContain("data-footnotes");
    expect(html).toContain("脚注");
    expect(html).not.toContain("sr-only");
  });

  it("サイト内リンクとサイト外リンクの href を保つ", () => {
    const html = render("[自己紹介](/about) と [外部](https://example.com)");
    expect(html).toContain('href="/about"');
    expect(html).toContain('href="https://example.com"');
  });

  it("許可しない HTML の要素は外し、script とコメントは中身ごと取り除く", () => {
    const html = render("<script>alert(1)</script>\n\n本文と<span>生</span>の HTML\n\n<!-- コメント -->");
    expect(html).not.toContain("<script>");
    expect(html).not.toContain("<span>");
    expect(html).not.toContain("alert(1)");
    expect(html).not.toContain("&lt;");
    expect(html).not.toContain("コメント");
    expect(html).toContain("本文と");
  });
});
