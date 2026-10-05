import { describe, expect, it } from "vitest";
import { cls, failures, inp, lcp, THRESHOLDS } from "./vitals.mjs";

describe("Core Web Vitals の計算（ADR 0050）", () => {
  it("LCP は最後の候補の時刻。候補がなければ null", () => {
    expect(lcp([{ startTime: 100 }, { startTime: 900 }])).toBe(900);
    expect(lcp([])).toBeNull();
  });

  it("CLS は、1 秒以内に続き 5 秒以内に収まるずれをまとめ、最大のまとまりを返す", () => {
    expect(cls([])).toBe(0);
    // 0.1 秒、0.5 秒、1.2 秒のずれは一つのまとまり（間がどれも 1 秒未満）。3 秒のずれは別のまとまり。
    expect(
      cls([
        { value: 0.01, startTime: 100, hadRecentInput: false },
        { value: 0.02, startTime: 500, hadRecentInput: false },
        { value: 0.03, startTime: 1200, hadRecentInput: false },
        { value: 0.05, startTime: 3000, hadRecentInput: false },
      ]),
    ).toBeCloseTo(0.06);
    // 間が 1 秒以上あけば、まとまりを分ける。
    expect(
      cls([
        { value: 0.04, startTime: 0, hadRecentInput: false },
        { value: 0.04, startTime: 1000, hadRecentInput: false },
      ]),
    ).toBeCloseTo(0.04);
    // 最初のずれから 5 秒を超えれば、間が短くても分ける。
    const steady = Array.from({ length: 12 }, (_, i) => ({ value: 0.01, startTime: i * 900, hadRecentInput: false }));
    expect(cls(steady)).toBeCloseTo(0.06);
    // 入力の直後のずれは数えない。順番が前後していても、時刻の順にまとめる。
    expect(
      cls([
        { value: 0.5, startTime: 200, hadRecentInput: true },
        { value: 0.02, startTime: 600, hadRecentInput: false },
        { value: 0.01, startTime: 300, hadRecentInput: false },
      ]),
    ).toBeCloseTo(0.03);
  });

  it("INP は、操作ごとに最も長いイベントを取り、そのうち最も長いものを返す", () => {
    expect(inp([])).toBeNull();
    // interactionId のないイベント（操作でないもの）は数えない。
    expect(inp([{ interactionId: 0, duration: 500 }])).toBeNull();
    expect(
      inp([
        { interactionId: 1, duration: 40 },
        { interactionId: 1, duration: 120 },
        { interactionId: 2, duration: 80 },
      ]),
    ).toBe(120);
  });

  it("INP は、操作が 50 回を超えると、50 回ごとに最も長いものを一つずつ除く", () => {
    const events = Array.from({ length: 120 }, (_, i) => ({ interactionId: i + 1, duration: i + 1 }));
    // 120 回なら、最も長い 2 回（120、119）を除いた 118。
    expect(inp(events)).toBe(118);
    expect(inp(events.slice(0, 49))).toBe(49);
    expect(inp(events.slice(0, 50))).toBe(49);
  });

  it("「良好」の上限を超えた値と、計測できなかった値を失敗にする", () => {
    expect(failures({ lcp: 2500, inp: 200, cls: 0.1 })).toEqual([]);
    expect(failures({ lcp: 2501, inp: 201, cls: 0.11 })).toEqual([
      { name: "lcp", value: 2501, limit: THRESHOLDS.lcp },
      { name: "inp", value: 201, limit: THRESHOLDS.inp },
      { name: "cls", value: 0.11, limit: THRESHOLDS.cls },
    ]);
    expect(failures({ lcp: null, inp: null, cls: 0 }).map((f) => f.name)).toEqual(["lcp", "inp"]);
  });
});
