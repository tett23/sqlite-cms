import { describe, expect, it } from "vitest";
import { isExternal } from "./externalLink";

describe("isExternal", () => {
  const origin = "https://blog.example.com";

  it("ほかのオリジンへの http と https のリンクを外とみなす", () => {
    expect(isExternal("https://zenn.dev/a", origin)).toBe(true);
    expect(isExternal("http://blog.example.com/a", origin)).toBe(true);
    expect(isExternal("https://blog.example.com:8443/a", origin)).toBe(true);
    expect(isExternal("HTTPS://ZENN.DEV/", origin)).toBe(true);
  });

  it("同じオリジン、サイト内のパス、ページ内のリンク、http と https 以外は外とみなさない", () => {
    for (const href of ["https://blog.example.com/articles/a", "/about", "about", "#fn-1", "mailto:a@example.com", "javascript:void(0)", "//zenn.dev/a"]) {
      expect(isExternal(href, origin), href).toBe(false);
    }
    expect(isExternal(undefined, origin)).toBe(false);
  });

  it("オリジンが分からないときは、http と https をすべて外とみなす", () => {
    expect(isExternal("https://blog.example.com/a", undefined)).toBe(true);
    expect(isExternal("/about", undefined)).toBe(false);
  });
});
