// Lighthouse で計測し、E2E のテストで開くページ（名前と、サイトの中のパス）。
export const PAGES = [
  ["top", "/"],
  ["article-syntax", "/articles/syntax"],
  ["article-extensions", "/articles/extensions"],
  ["article-getting-started", "/articles/getting-started"],
  ["post-ruby", "/posts/ruby"],
  ["about", "/about"],
  ["search", "/search?q=%E8%A8%98%E4%BA%8B"],
  ["archive", "/archive"],
  // 重いページの見本（コード、数式、図がそれぞれ多い記事と、長い記事）。
  ["heavy-code", "/articles/heavy-code"],
  ["heavy-math", "/articles/heavy-math"],
  ["heavy-diagrams", "/articles/heavy-diagrams"],
  ["heavy-long", "/articles/heavy-long"],
  // 複雑なページの見本（入れ子の深い記事と、コード、数式、図を混ぜた記事）。
  ["complex-nesting", "/articles/complex-nesting"],
  ["complex-mixed", "/articles/complex-mixed"],
];
