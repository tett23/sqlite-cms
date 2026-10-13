import { describe, expect, it, vi } from "vitest";
import { formatResult, runScenario } from "./scenarios.mjs";

const unresponsive = (message = "Chrome が 30 秒応えない: Runtime.evaluate") => Object.assign(new Error(message), { unresponsive: true });

function run(fn) {
  const tabs = [];
  const openTab = vi.fn(async () => {
    const tab = { close: vi.fn(async () => {}) };
    tabs.push(tab);
    return tab;
  });
  return { tabs, openTab, result: runScenario({ name: "場面", fn }, { openTab, args: (tab) => ({ tab }), timeout: 1000 }) };
}

describe("runScenario（ADR 0071）", () => {
  it("Chrome が応えずに失敗した場面は、新しいタブで一度やり直し、通れば成功に数えてやり直したことを書く", async () => {
    const fn = vi.fn().mockRejectedValueOnce(unresponsive()).mockResolvedValueOnce(undefined);
    const { tabs, result } = run(fn);
    const r = await result;
    expect(r).toMatchObject({ ok: true, retried: "Chrome が 30 秒応えない: Runtime.evaluate" });
    expect(tabs).toHaveLength(2);
    expect(fn.mock.calls.map(([args]) => args.tab)).toEqual(tabs);
    for (const tab of tabs) expect(tab.close).toHaveBeenCalledTimes(1);
    expect(formatResult({ ...r, ms: 1500 })).toBe("✓ 場面（1.5 秒）（Chrome が応えず、やり直して通った）\n    1 回目: Chrome が 30 秒応えない: Runtime.evaluate");
  });

  it("条件が成り立たない失敗は、やり直さない", async () => {
    const fn = vi.fn().mockRejectedValue(new Error("待っても成り立たない: 本文"));
    const { tabs, result } = run(fn);
    expect(await result).toMatchObject({ ok: false, error: "待っても成り立たない: 本文" });
    expect(tabs).toHaveLength(1);
  });

  it("やり直しても応えなければ、失敗にする", async () => {
    const fn = vi.fn().mockRejectedValueOnce(unresponsive("一回目")).mockRejectedValueOnce(unresponsive("二回目"));
    const { result } = run(fn);
    const r = await result;
    expect(r).toMatchObject({ ok: false, error: "二回目", retried: "一回目" });
    expect(formatResult({ ...r, ms: 2000 })).toBe("✗ 場面（2.0 秒）（Chrome が応えず、やり直しても失敗した）\n    1 回目: 一回目\n    二回目");
  });

  it("時間内に終わらない場面は、やり直さずに失敗にする", async () => {
    const { result } = run(() => new Promise(() => {}));
    expect(await result).toMatchObject({ ok: false, error: "1 秒で終わらない" });
  });

  it("通った場面と失敗した場面の一行", () => {
    expect(formatResult({ name: "a", ok: true, ms: 200 })).toBe("✓ a（0.2 秒）");
    expect(formatResult({ name: "a", ok: false, ms: 200, error: "x" })).toBe("✗ a（0.2 秒）\n    x");
  });
});
