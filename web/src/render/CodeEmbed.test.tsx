import { renderToStaticMarkup } from "preact-render-to-string";
import { describe, expect, it } from "vitest";
import { MarkdownBody } from "../MarkdownBody";
import { lineNumberStyle, sourceLabel } from "./CodeEmbed";

const FILE = "https://github.com/o/r/blob/0123abc/src/a.rs";
const file = (startLine: number, content: string) => ({ fileName: "a.rs", sourceUrl: FILE, startLine, content });

describe("コードの埋め込みの見出し（ADR 0065）", () => {
  it("行の範囲がなければ、リポジトリだけを書く", () => {
    expect(sourceLabel(FILE, file(1, "a\nb"))).toBe("o/r");
  });

  it("一行だけの範囲は、その行だけを書く", () => {
    expect(sourceLabel(`${FILE}#L10`, file(10, "a"))).toBe("o/r L10");
  });

  it("範囲の終わりは、実際に表示する最後の行にする（ファイルの終わりで切り詰めたとき）", () => {
    expect(sourceLabel(`${FILE}#L10-L99`, file(10, "a\nb\nc"))).toBe("o/r L10-L12");
  });

  it("Gist は Gist と書く", () => {
    expect(sourceLabel("https://gist.github.com/u/0123", file(1, "a"))).toBe("Gist");
  });
});

describe("コードの埋め込みの行番号（ADR 0065）", () => {
  it("番号の欄の幅は、最後の行の番号の桁数に合わせる", () => {
    expect(lineNumberStyle(98, 5)).toEqual({ counterReset: "line 97", "--line-number-width": "3ch" });
    expect(lineNumberStyle(1, 9)).toEqual({ counterReset: "line 0", "--line-number-width": "1ch" });
    expect(lineNumberStyle(1, 10)).toEqual({ counterReset: "line 0", "--line-number-width": "2ch" });
    // 中身が空でも、一行として幅を決める。
    expect(lineNumberStyle(10, 0)).toEqual({ counterReset: "line 9", "--line-number-width": "2ch" });
  });

  it("色を付ける前も、空の行を含めて一行ずつ span.line で囲む（番号がずれない）", () => {
    const embeds = new Map([[FILE, [file(1, "fn a() {}\n\nfn b() {}")]]]);
    const html = renderToStaticMarkup(<MarkdownBody source={FILE} codeEmbeds={embeds} />);
    expect(html.match(/<span class="line">/g)).toHaveLength(3);
    expect(html).toContain('<span class="line"></span>');
  });

  it("リストと引用の中の URL だけの行も、中身を表示する", () => {
    const embeds = new Map([[FILE, [file(1, "fn a() {}")]]]);
    for (const source of [`- ${FILE}`, `> ${FILE}`]) {
      const html = renderToStaticMarkup(<MarkdownBody source={source} codeEmbeds={embeds} />);
      expect(html, source).toContain('<figure class="code-embed">');
      expect(html, source).not.toContain("link-card-slot");
    }
  });
});
