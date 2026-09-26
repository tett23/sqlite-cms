import { describe, expect, it } from "vitest";
import { pageDescription, setMetaDescription } from "./documentMeta";

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

/** head の meta だけを持つ、小さな偽の document。 */
function fakeDocument(existing?: { name: string; content: string }) {
  const metas: { name: string; content: string }[] = existing ? [existing] : [];
  const doc = {
    head: {
      querySelector: (selector: string) =>
        selector === 'meta[name="description"]' ? (metas.find((m) => m.name === "description") ?? null) : null,
      appendChild: (meta: { name: string; content: string }) => metas.push(meta),
    },
    createElement: () => ({ name: "", content: "" }),
  };
  return { doc: doc as unknown as Document, metas };
}

describe("setMetaDescription", () => {
  it("meta がなければ作り、あれば書き換える（増やさない）", () => {
    const { doc, metas } = fakeDocument();
    setMetaDescription(doc, "一つめ");
    setMetaDescription(doc, "二つめ");
    expect(metas).toEqual([{ name: "description", content: "二つめ" }]);

    const existing = fakeDocument({ name: "description", content: "元" });
    setMetaDescription(existing.doc, "新");
    expect(existing.metas).toEqual([{ name: "description", content: "新" }]);
  });
});
