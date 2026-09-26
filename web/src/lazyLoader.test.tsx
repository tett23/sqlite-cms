import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { highlightLoader, type Highlight } from "./highlightLoader";
import { createLoader } from "./lazyLoader";
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
