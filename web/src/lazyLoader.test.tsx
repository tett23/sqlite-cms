import { renderToStaticMarkup } from "preact-render-to-string";
import { afterEach, describe, expect, it, vi } from "vitest";
import { highlightLoader, type Highlight } from "./highlightLoader";
import { createLoader, inTurn, whenIdle } from "./lazyLoader";
import { MarkdownBody } from "./MarkdownBody";

const fake: Highlight = { highlightBlock: () => null };

describe("createLoader", () => {
  it("読み込むまでは null で、読み込み後は結果を返す", async () => {
    const loader = createLoader(async () => fake);
    expect(loader.loaded()).toBeNull();
    await expect(loader.load()).resolves.toBe(fake);
    expect(loader.loaded()).toBe(fake);
  });

  it("同時に呼んでも読み込みは一度だけ", async () => {
    let calls = 0;
    const loader = createLoader(async () => {
      calls += 1;
      return fake;
    });
    const [a, b] = await Promise.all([loader.load(), loader.load()]);
    await loader.load();
    expect(a).toBe(b);
    expect(calls).toBe(1);
  });

  it("失敗したら次の呼び出しで読み込み直す", async () => {
    let calls = 0;
    const loader = createLoader(async () => {
      calls += 1;
      if (calls === 1) throw new Error("network");
      return fake;
    });
    await expect(loader.load()).rejects.toThrow("network");
    expect(loader.loaded()).toBeNull();
    await expect(loader.load()).resolves.toBe(fake);
    expect(calls).toBe(2);
  });
});

describe("読み込み終わりの通知", () => {
  it("読み込み終わったら登録した関数を呼び、登録を解いたら呼ばない", async () => {
    const loader = createLoader(async () => fake);
    const calls: string[] = [];
    const unsubscribe = loader.subscribe(() => calls.push("a"));
    loader.subscribe(() => calls.push("b"));
    unsubscribe();
    await loader.load();
    expect(calls).toEqual(["b"]);
  });

  it("失敗したときは呼ばない", async () => {
    const loader = createLoader<Highlight>(async () => {
      throw new Error("network");
    });
    let called = false;
    loader.subscribe(() => (called = true));
    await expect(loader.load()).rejects.toThrow("network");
    expect(called).toBe(false);
  });
});

describe("MarkdownBody の非同期のハイライト", () => {
  const source = "```rust\nfn main() {}\n```";

  it("読み込む前は色なしで描画し、読み込んだ後は色を付ける", async () => {
    expect(highlightLoader.loaded()).toBeNull();
    const before = renderToStaticMarkup(<MarkdownBody source={source} />);
    expect(before).toBe(
      '<div class="article-body mt-6"><pre><code class="language-rust">fn main() {}\n</code></pre></div>',
    );

    await highlightLoader.load();
    const after = renderToStaticMarkup(<MarkdownBody source={source} />);
    expect(after).toContain('class="shiki github-light-high-contrast"');
  });
});

describe("一つずつ別のタスクで呼ぶ（inTurn、ADR 0038）", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("登録した順に、タスクごとに一つずつ呼ぶ", () => {
    vi.useFakeTimers();
    const calls: string[] = [];
    inTurn(() => calls.push("a"));
    inTurn(() => calls.push("b"));
    inTurn(() => calls.push("c"));
    expect(calls).toEqual([]);
    vi.advanceTimersToNextTimer();
    expect(calls).toEqual(["a"]);
    vi.advanceTimersToNextTimer();
    expect(calls).toEqual(["a", "b"]);
    vi.runAllTimers();
    expect(calls).toEqual(["a", "b", "c"]);
  });

  it("取り消したものは呼ばず、残りは続けて呼ぶ", () => {
    vi.useFakeTimers();
    const calls: string[] = [];
    inTurn(() => calls.push("a"));
    const cancel = inTurn(() => calls.push("b"));
    inTurn(() => calls.push("c"));
    cancel();
    vi.runAllTimers();
    expect(calls).toEqual(["a", "c"]);
  });

  it("すべて呼び終えた後に登録したものも呼ぶ", () => {
    vi.useFakeTimers();
    const calls: string[] = [];
    inTurn(() => calls.push("a"));
    vi.runAllTimers();
    inTurn(() => calls.push("b"));
    vi.runAllTimers();
    expect(calls).toEqual(["a", "b"]);
  });
});

describe("描画の後、手が空いたときに呼ぶ（whenIdle、ADR 0038）", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  /** requestAnimationFrame と requestIdleCallback を、呼ぶまで溜めておくものにする。 */
  function stubFrames() {
    const frames: (() => void)[] = [];
    const idles: (() => void)[] = [];
    vi.stubGlobal("requestAnimationFrame", (fn: () => void) => frames.push(fn));
    vi.stubGlobal("cancelAnimationFrame", (id: number) => (frames[id - 1] = () => {}));
    vi.stubGlobal("requestIdleCallback", (fn: () => void) => idles.push(fn));
    vi.stubGlobal("cancelIdleCallback", (id: number) => (idles[id - 1] = () => {}));
    return { frames, idles };
  }

  it("次の描画の後（requestAnimationFrame の後のタスク）に、手が空くのを待ってから呼ぶ", () => {
    vi.useFakeTimers();
    const { frames, idles } = stubFrames();
    const fn = vi.fn();
    whenIdle(fn);
    expect(frames).toHaveLength(1);
    expect(idles).toHaveLength(0);
    frames[0]();
    expect(idles).toHaveLength(0);
    vi.runAllTimers();
    expect(idles).toHaveLength(1);
    expect(fn).not.toHaveBeenCalled();
    idles[0]();
    expect(fn).toHaveBeenCalledOnce();
  });

  it("どの段階で取り消しても呼ばない", () => {
    vi.useFakeTimers();
    for (const stage of ["frame", "timer", "idle"] as const) {
      const { frames, idles } = stubFrames();
      const fn = vi.fn();
      const cancel = whenIdle(fn);
      if (stage !== "frame") frames[0]();
      if (stage === "idle") vi.runAllTimers();
      cancel();
      // まだ呼んでいないものだけを呼ぶ（取り消したものは何もしない関数に置き換わっている）。
      if (stage === "frame") frames[0]();
      vi.runAllTimers();
      idles.forEach((f) => f());
      expect(fn, stage).not.toHaveBeenCalled();
    }
  });

  it("requestAnimationFrame がなければ（サーバーでの描画など）、少し待ってから呼ぶ", () => {
    vi.useFakeTimers();
    const fn = vi.fn();
    whenIdle(fn);
    vi.advanceTimersByTime(199);
    expect(fn).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(fn).toHaveBeenCalledOnce();
  });
});
