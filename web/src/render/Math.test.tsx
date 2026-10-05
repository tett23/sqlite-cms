import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "preact-render-to-string";
import { displayMathHeight, MathView } from "./Math";

describe("MathView の組む前の表示", () => {
  it("文中の大きな分数だけ、行を高くしておく", () => {
    expect(renderToStaticMarkup(<MathView tex={"\\dfrac{a}{b}"} display={false} />)).toBe(
      '<span style="line-height:2.5;" class="math-inline">\\dfrac{a}{b}</span>',
    );
    expect(renderToStaticMarkup(<MathView tex={"\\frac{a}{b}"} display={false} />)).toBe('<span class="math-inline">\\frac{a}{b}</span>');
  });
});

describe("displayMathHeight", () => {
  it("一行の数式は、分数などがあれば高く見積もる", () => {
    expect(displayMathHeight("e^{i\\theta} = \\cos\\theta + i\\sin\\theta")).toBeCloseTo(3.75);
    expect(displayMathHeight("F = G \\frac{m_1 m_2}{r^2}")).toBeCloseTo(4.9);
    expect(displayMathHeight("\\sum_{n=0}^{\\infty} r^n")).toBeCloseTo(4.9);
    // \fraction のような別の命令は、分数とみなさない。
    expect(displayMathHeight("\\fraction")).toBeCloseTo(3.75);
  });

  it("行を分ける環境は、行の数だけ高く見積もる", () => {
    expect(displayMathHeight("\\begin{aligned}\na &= \\frac{1}{2} \\\\\nb &= \\frac{3}{4} \\\\\nc &= 1\n\\end{aligned}")).toBeCloseTo(2 + 3 * 2.9);
    expect(displayMathHeight("\\begin{cases}\nx = 1 \\\\\ny = 2\n\\end{cases}")).toBeCloseTo(2 + 2 * 1.75);
  });

  it("行列の \\\\ は、行を分ける環境の外なら一行とみなす", () => {
    expect(displayMathHeight("\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}")).toBeCloseTo(4.9);
  });
});
