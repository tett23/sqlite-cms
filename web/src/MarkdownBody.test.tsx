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

  it("サイトの外へのリンクは新しいタブで開き、サイト内とページ内のリンクはそのまま（ADR 0057）", () => {
    expect(render("[外](https://zenn.dev/)")).toContain('<a href="https://zenn.dev/" target="_blank" rel="noopener">外</a>');
    expect(render('<a href="https://zenn.dev/">HTML で書いた外</a>')).toContain('<a href="https://zenn.dev/" target="_blank" rel="noopener">');
    expect(render("[内](/about)")).toContain('<a href="/about">内</a>');
    expect(render("[相対](about)")).toContain('<a href="about">相対</a>');
    expect(render("[メール](mailto:a@example.com)")).toContain('<a href="mailto:a@example.com">メール</a>');
    expect(render("本文[^1]\n\n[^1]: 注。")).not.toContain("target=");
    // 書き手が HTML に書いた target と rel は取り除く（サイト内のリンクを新しいタブで開かせない）。
    expect(render('<a href="/about" target="_blank" rel="opener">内</a>')).toContain('<a href="/about">内</a>');
  });

  it("GFM の自動リンクを描画する", () => {
    expect(render("www.example.com")).toContain('<a href="http://www.example.com" target="_blank" rel="noopener">');
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

describe("ヘッダの Markdown（ADR 0043）", () => {
  const header = '[記事置き場](/)\n\n- [一覧](/archive)\n\n<div class="partial-search"></div>\n';

  it("パーシャルの目印を、渡した部品に置き換える", () => {
    const html = renderToStaticMarkup(
      <MarkdownBody source={header} baseClassName="site-header-body" className="" partials={{ search: <form role="search" /> }} />,
    );
    expect(html).toBe(
      '<div class="site-header-body"><p><a href="/">記事置き場</a></p>\n<ul>\n<li><a href="/archive">一覧</a></li>\n</ul>\n<form role="search"></form></div>',
    );
  });

  it("部品を渡さなければ、目印は何も描かない", () => {
    expect(render('<div class="partial-search"></div>')).toBe('<div class="article-body mt-6"></div>');
  });
});
