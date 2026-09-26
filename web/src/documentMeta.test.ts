import { describe, expect, it } from "vitest";
import { pageDescription } from "./documentMeta";

describe("pageDescription", () => {
  it("ページ自身の説明があればそれを使う", () => {
    expect(pageDescription("サイトの説明", "記事の要約")).toBe("記事の要約");
  });

  it("ページ自身の説明がなければサイトの説明を使う", () => {
    expect(pageDescription("サイトの説明")).toBe("サイトの説明");
    expect(pageDescription("サイトの説明", null)).toBe("サイトの説明");
    expect(pageDescription("サイトの説明", "  ")).toBe("サイトの説明");
  });
});
