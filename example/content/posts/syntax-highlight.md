---
title: "コードの色分け"
date: 2026-09-25
---

コードブロックに言語名を書くと、色が付く。
色分けには Shiki を使っていて、VS Code と同じ文法の定義で色を決めている。

```haskell
-- 偶数だけを 2 倍にして表示する
main :: IO ()
main = mapM_ print [x * 2 | x <- [1 .. 10 :: Int], even x]
```

```rust
fn first_word<'a>(s: &'a str) -> Option<&'a str> {
    s.split_whitespace().next()
}
```

```typescript
type Pick2<T, K extends keyof T> = { [P in K]: T[P] };

export function getTitle(post: { title: string }): string {
  return post.title.trim();
}
```

```tsx
export const List = ({ items }: { items: string[] }) => (
  <ul>
    {items.map((item) => (
      <li key={item}>{item}</li>
    ))}
  </ul>
);
```

```toml
title = "tett23の記事置き場"

[deploy]
worker = "my-blog"
```

```sh
sqlite-cms new post hello --title はじめまして
sqlite-cms serve
```

```diff
- date: 2026-02-30
+ date: 2026-02-28
```

言語名を書かないブロックと、対応していない言語のブロックは、色なしで表示する。

```
言語名のないブロック
```

```brainfuck
++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.
```
