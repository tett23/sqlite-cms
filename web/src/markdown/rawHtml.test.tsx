import { renderToStaticMarkup } from "preact-render-to-string";
import { describe, expect, it } from "vitest";
import { MarkdownBody } from "../MarkdownBody";
import { decodeEntities, tokenize } from "./rawHtml";

const render = (source: string) =>
  renderToStaticMarkup(<MarkdownBody source={source} />).replace(/^<div class="article-body mt-6">|<\/div>$/g, "");

describe("HTML を字句に分ける（ADR 0046）", () => {
  it("開始タグ、終了タグ、文字に分け、タグの名前と属性の名前を小文字にする", () => {
    expect(tokenize('<KBD Title="a">x</kbd>')).toEqual([
      { type: "start", tagName: "kbd", attributes: { title: "a" }, selfClosing: false },
      { type: "text", value: "x" },
      { type: "end", tagName: "kbd" },
    ]);
  });

  it("属性は二重引用符、一重引用符、引用符なし、値なしで書ける。同じ名前は最初のものを使う", () => {
    expect(tokenize(`<details open title='t "q"' data-x=1 title="2">`)).toEqual([
      { type: "start", tagName: "details", attributes: { open: "", title: 't "q"', "data-x": "1" }, selfClosing: false },
    ]);
  });

  it("注釈、doctype、CDATA は捨てる", () => {
    expect(tokenize("a<!-- <b> -->b<!doctype html><![CDATA[x]]>c")).toEqual([
      { type: "text", value: "a" },
      { type: "text", value: "b" },
      { type: "text", value: "c" },
    ]);
  });

  it("script と style の中はタグとして読まない", () => {
    expect(tokenize("<script>if (a < b) { x = '</div>' }</script>after")).toEqual([
      { type: "start", tagName: "script", attributes: {}, selfClosing: false },
      { type: "text", value: "if (a < b) { x = '</div>' }" },
      { type: "end", tagName: "script" },
      { type: "text", value: "after" },
    ]);
    expect(tokenize("<STYLE>a > b {}</Style>")).toEqual([
      { type: "start", tagName: "style", attributes: {}, selfClosing: false },
      { type: "text", value: "a > b {}" },
      { type: "end", tagName: "style" },
    ]);
  });

  it("タグでない < と、閉じていないタグは文字にする", () => {
    expect(tokenize("1 < 2 <3")).toEqual([
      { type: "text", value: "1 " },
      { type: "text", value: "<" },
      { type: "text", value: " 2 " },
      { type: "text", value: "<" },
      { type: "text", value: "3" },
    ]);
    expect(tokenize('<a href="x"')).toEqual([{ type: "text", value: '<a href="x"' }]);
  });

  it("文字参照を文字にし、知らない名前はそのまま残す", () => {
    expect(decodeEntities("&amp;&lt;&gt;&quot;&#39;&#x3042;&nbsp;&copy;&unknown;&#0;")).toBe("&<>\"'あ ©&unknown;�");
  });
});

describe("HTML を要素として取り込む（ADR 0046）", () => {
  it("開始と終了の間の Markdown の段落を、要素の子にする", () => {
    expect(render("<details>\n<summary>補足</summary>\n\n本文の **段落**\n\n</details>")).toBe(
      "<details>\n<summary>補足</summary>\n<p>本文の <strong>段落</strong></p>\n</details>",
    );
  });

  it("段落の中の HTML も取り込む", () => {
    expect(render("<kbd>Ctrl</kbd> + <ruby>漢字<rt>かんじ</rt></ruby>")).toBe("<p><kbd>Ctrl</kbd> + <ruby>漢字<rt>かんじ</rt></ruby></p>");
  });

  it("入れ子にした要素を閉じる", () => {
    expect(render("<dl>\n<dt>用語</dt>\n<dd>説明<br>次の行</dd>\n</dl>")).toBe("<dl>\n<dt>用語</dt>\n<dd>説明<br/>次の行</dd>\n</dl>");
  });

  it("閉じていない要素は親の終わりで閉じ、開いていない終了タグは捨てる", () => {
    expect(render("<kbd>閉じない\n\n次の段落</kbd>")).toBe("<p><kbd>閉じない</kbd></p>\n<p>次の段落</p>");
    expect(render("a</kbd>b")).toBe("<p>ab</p>");
  });

  it("許可しない要素と属性は、取り込んだ後の無害化で消える", () => {
    expect(render('<kbd onclick="alert(1)" style="color:red">k</kbd><script>alert(1)</script>')).toBe("<p><kbd>k</kbd></p>");
    expect(render('<a href="javascript:alert(1)">x</a>')).toBe("<p><a>x</a></p>");
  });
});
