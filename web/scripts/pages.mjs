// Lighthouse で計測し、E2E のテストで開くページ（名前と、サイトの中のパス）。
export const PAGES = [
  ["top", "/"],
  ["article-syntax", "/articles/syntax"],
  ["article-extensions", "/articles/extensions"],
  ["article-getting-started", "/articles/getting-started"],
  ["post-ruby", "/posts/01a0cb8f-e400-7083-945c-cc81919f7840"],
  ["about", "/about"],
  ["search", "/search?q=%E8%A8%98%E4%BA%8B"],
  ["archive", "/archive"],
  ["category", "/howto"],
  // 重いページの見本（コード、数式、図がそれぞれ多い記事と、長い記事）。
  ["heavy-code", "/articles/heavy-code"],
  ["heavy-math", "/articles/heavy-math"],
  ["heavy-diagrams", "/articles/heavy-diagrams"],
  ["heavy-long", "/articles/heavy-long"],
  // 複雑なページの見本（入れ子の深い記事と、コード、数式、図を混ぜた記事）。
  ["complex-nesting", "/articles/complex-nesting"],
  ["complex-mixed", "/articles/complex-mixed"],
];

/**
 * ページを count に分けた index 番目（1 から数える）だけを返す（ADR 0052）。spec は "index/count"。なければすべてを返す。
 * CI で、Core Web Vitals の計測を別々の機械で並行して行うために使う。同じ機械で並行すると、CPU を取り合って値が揺れる。
 */
export function shard(pages, spec) {
  if (!spec) return pages;
  const match = /^(\d+)\/(\d+)$/.exec(spec);
  const [index, count] = match ? [Number(match[1]), Number(match[2])] : [0, 0];
  if (count < 1 || index < 1 || index > count) throw new Error(`分け方は「番号/数」で書く（例 1/2）: ${spec}`);
  return pages.filter((_, i) => i % count === index - 1);
}
