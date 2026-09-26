import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { createHighlightLoader, highlightLoader, type Highlight } from "./highlightLoader";
import { MarkdownBody } from "./MarkdownBody";

const fake: Highlight = { rehypePlugins: [] };

describe("createHighlightLoader", () => {
  it("読み込むまでは null で、読み込み後は結果を返す", async () => {
    const loader = createHighlightLoader(async () => fake);
    expect(loader.loaded()).toBeNull();
    await expect(loader.load()).resolves.toBe(fake);
    expect(loader.loaded()).toBe(fake);
  });

  it("同時に呼んでも読み込みは一度だけ", async () => {
    let calls = 0;
    const loader = createHighlightLoader(async () => {
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
    const loader = createHighlightLoader(async () => {
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
