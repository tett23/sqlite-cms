// リンクカードの部品を読み込む前の表示（ADR 0049）。読み込みの状態はファイルごとに分かれるので、ほかのテストと分ける。
import { renderToStaticMarkup } from "preact-render-to-string";
import { describe, expect, it } from "vitest";
import { MarkdownBody } from "../MarkdownBody";
import { linkCardLoader } from "../render/loaders";

const render = (source: string) =>
  renderToStaticMarkup(<MarkdownBody source={source} />).replace(/^<div class="article-body mt-6">|<\/div>$/g, "");

describe("リンクカードの部品を読み込む前", () => {
  it("URL だけの段落は、カードを置く場所の中に、普通のリンクとして出す。部品は読み込まない", () => {
    expect(render("https://example.com/path")).toBe(
      '<div class="link-card-slot"><a href="https://example.com/path">https://example.com/path</a></div>',
    );
    // サーバーでの描画では読み込みを始めない（ブラウザでは、画面の近くに来てから読み込む）。
    expect(linkCardLoader.loaded()).toBeNull();
  });

  it("文中の URL と、リンク文字列のあるリンクは、カードを置く場所にしない", () => {
    expect(render("文中の https://example.com です")).not.toContain("link-card");
    expect(render("[例](https://example.com)")).not.toContain("link-card");
  });

  it("http と https のほかの URL は、カードを置く場所にしない", () => {
    expect(render("<mailto:a@example.com>")).not.toContain("link-card");
  });
});
