import { renderToStaticMarkup } from "preact-render-to-string";
import { describe, expect, it } from "vitest";
import { MarkdownBody } from "../MarkdownBody";
import { firstPart } from "./firstPart";

/** 先頭の段落（min 文字を超える）と、その後の見出しと段落。 */
function doc(between: string, min = 20): string {
  return `${"あ".repeat(min)}\n\n${between}\n\n## 見出し\n\n後ろ。\n`;
}

describe("firstPart", () => {
  it("最低の文字数より後の、空行の後の見出しの直前で切る", () => {
    const source = "## 一\n\n短い。\n\n## 二\n\n" + "い".repeat(30) + "\n\n## 三\n\n後ろ。\n";
    expect(firstPart(source, 20)).toBe("## 一\n\n短い。\n\n## 二\n\n" + "い".repeat(30) + "\n\n");
  });

  it("短い本文、見出しのない本文は切らない", () => {
    expect(firstPart("## 一\n\n短い。\n", 20)).toBeNull();
    expect(firstPart("あ".repeat(50), 20)).toBeNull();
  });

  it("後ろにある一行の定義は、先頭の部分の後ろに足す", () => {
    const head = `${"あ".repeat(20)}\n\n[参照][a]と注[^1]\n\n`;
    expect(firstPart(`${head}## 見出し\n\n[a]: https://example.com/\n[^1]: 注。\n\n後ろ。\n`, 20)).toBe(
      `${head}[a]: https://example.com/\n\n[^1]: 注。\n`,
    );
  });

  it("足せない形の定義が後ろにあれば切らない", () => {
    for (const rest of [
      "[^1]: 注の\n    続き。",
      "段落\n[a]: https://example.com/",
      "> [a]: https://example.com/",
      "- [a]: https://example.com/",
      ":::message\n[a]: https://example.com/\n:::",
    ]) {
      expect(firstPart(doc("本文") + `\n${rest}\n`, 20), rest).toBeNull();
    }
  });

  it("コードの中の定義のような行は足さない", () => {
    expect(firstPart(doc("本文") + "\n```\n[a]: https://example.com/\n```\n", 20)).toBe(`${"あ".repeat(20)}\n\n本文\n\n`);
  });

  it("コードのフェンス、数式、:::、空行で終わらない HTML の中では切らない", () => {
    for (const block of [
      "```md\n\n## コードの中\n```",
      "~~~~\n```\n\n## コードの中\n~~~~",
      "$$\nx\n\n# 数式の中\n$$",
      ":::message\n\n## 中\n\n:::",
      "::::details 外\n:::message\n中\n:::\n\n## 外の中\n\n::::",
      "<pre>\n\n## pre の中\n</pre>",
      "<!--\n\n## コメントの中\n-->",
    ]) {
      const part = firstPart(doc(block), 20);
      expect(part, block).toBe(`${"あ".repeat(20)}\n\n${block}\n\n`);
    }
  });

  it("字下げした見出し、空行の後でない見出しでは切らない", () => {
    expect(firstPart(doc("- 項目\n\n    # 字下げ"), 20)).toBe(`${"あ".repeat(20)}\n\n- 項目\n\n    # 字下げ\n\n`);
    expect(firstPart(`${"あ".repeat(20)}\n# 段落の直後\n\n## 見出し\n`, 10)).toBe(`${"あ".repeat(20)}\n# 段落の直後\n\n`);
  });

  it("見本の記事では、先頭の部分を描いた結果が、全体を描いた結果の先頭と一致する", () => {
    const articles = import.meta.glob<string>("../../../example/content/**/*.md", { query: "?raw", import: "default", eager: true });
    let cut = 0;
    for (const [file, raw] of Object.entries(articles)) {
      const body = raw.replace(/^---\n[\s\S]*?\n---\n/, "");
      for (const min of [200, 2000]) {
        const part = firstPart(body, min);
        if (part === null) continue;
        cut++;
        const whole = renderToStaticMarkup(<MarkdownBody source={body} />);
        // 定義を足したので、先頭の部分にも脚注の欄が出る（少しずつ描くときは描かない）。
        const first = renderToStaticMarkup(<MarkdownBody source={part} />)
          .replace(/<\/div>$/, "")
          .replace(/<section data-footnotes[\s\S]*<\/section>\n?$/, "");
        expect(whole.startsWith(first), `${file}（${min} 字）`).toBe(true);
      }
    }
    expect(cut).toBeGreaterThan(5);
  });
});
