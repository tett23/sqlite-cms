import { describe, expect, it } from "vitest";
import { PAGES, shard } from "./pages.mjs";

describe("shard", () => {
  it("ページを重ならずに分け、合わせるとすべてになる", () => {
    const parts = [shard(PAGES, "1/2"), shard(PAGES, "2/2")];
    expect(parts[0].length + parts[1].length).toBe(PAGES.length);
    expect(new Set([...parts[0], ...parts[1]]).size).toBe(PAGES.length);
    // 重いページが片方に偏らないよう、交互に分ける。
    expect(parts[0][0]).toBe(PAGES[0]);
    expect(parts[1][0]).toBe(PAGES[1]);
  });

  it("指定がなければすべてを返し、書き誤りはエラーにする", () => {
    expect(shard(PAGES, undefined)).toBe(PAGES);
    for (const spec of ["0/2", "3/2", "1/0", "1", "a/b"]) expect(() => shard(PAGES, spec), spec).toThrow();
  });
});
