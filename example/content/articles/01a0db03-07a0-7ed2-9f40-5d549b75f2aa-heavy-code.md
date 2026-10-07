---
title: "重いページの見本：コードの多い記事"
slug: heavy-code
date: 2026-09-26
description: 表示の重さを確かめるための見本。色分けするすべての言語で、同じ処理を書いたコードブロックを 40 個ほど並べる。
tags: [見本, 計測, コード]
---

:::message
このページは、表示の重さを確かめるための見本です。
コードブロックが多い記事で、色分け（Shiki）の読み込みと処理が、本文の表示や入力への応答をどれだけ遅らせるかを Lighthouse で計測しています。
:::

同じ三つの処理を、いろいろな言語で書きます。

1. **FizzBuzz**：1 から 15 までの数を、3 の倍数なら `Fizz`、5 の倍数なら `Buzz`、両方の倍数なら `FizzBuzz` に置き換えて出力する
2. **二分探索**：整列した配列から値を探し、その位置を返す
3. **語の数え上げ**：文章を空白で区切り、語ごとの出現回数を数える

その後に、設定ファイルやマークアップなど、処理を書かない言語の見本を並べます。

## Rust

```rust:fizzbuzz.rs
fn fizzbuzz(n: u32) -> String {
    match (n % 3, n % 5) {
        (0, 0) => "FizzBuzz".to_string(),
        (0, _) => "Fizz".to_string(),
        (_, 0) => "Buzz".to_string(),
        _ => n.to_string(),
    }
}

fn main() {
    for n in 1..=15 {
        println!("{}", fizzbuzz(n));
    }
}
```

```rust:search.rs
/// 整列した slice から target を探し、見つかればその位置を返す。
fn binary_search(items: &[i64], target: i64) -> Option<usize> {
    let (mut low, mut high) = (0, items.len());
    while low < high {
        let mid = low + (high - low) / 2;
        match items[mid].cmp(&target) {
            std::cmp::Ordering::Less => low = mid + 1,
            std::cmp::Ordering::Greater => high = mid,
            std::cmp::Ordering::Equal => return Some(mid),
        }
    }
    None
}
```

```rust:count.rs
use std::collections::BTreeMap;

fn count_words(text: &str) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    for word in text.split_whitespace() {
        *counts.entry(word).or_insert(0) += 1;
    }
    counts
}

fn main() {
    for (word, count) in count_words("the cat and the hat") {
        println!("{word}: {count}");
    }
}
```

少し長い例として、四則演算の式を読んで計算するプログラムを書きます。

```rust:calc.rs
//! 四則演算と括弧を読んで計算する。
//! expr   = term (("+" | "-") term)*
//! term   = factor (("*" | "/") factor)*
//! factor = number | "(" expr ")" | "-" factor

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' => {}
            '+' => tokens.push(Token::Plus),
            '-' => tokens.push(Token::Minus),
            '*' => tokens.push(Token::Star),
            '/' => tokens.push(Token::Slash),
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            '0'..='9' | '.' => {
                let start = i;
                while i + 1 < chars.len() && (chars[i + 1].is_ascii_digit() || chars[i + 1] == '.') {
                    i += 1;
                }
                let text: String = chars[start..=i].iter().collect();
                let value = text.parse().map_err(|_| format!("数として読めない: {text}"))?;
                tokens.push(Token::Number(value));
            }
            _ => return Err(format!("知らない文字: {c}")),
        }
        i += 1;
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        token
    }

    fn expr(&mut self) -> Result<f64, String> {
        let mut value = self.term()?;
        while let Some(token) = self.peek() {
            match token {
                Token::Plus => {
                    self.next();
                    value += self.term()?;
                }
                Token::Minus => {
                    self.next();
                    value -= self.term()?;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn term(&mut self) -> Result<f64, String> {
        let mut value = self.factor()?;
        while let Some(token) = self.peek() {
            match token {
                Token::Star => {
                    self.next();
                    value *= self.factor()?;
                }
                Token::Slash => {
                    self.next();
                    let divisor = self.factor()?;
                    if divisor == 0.0 {
                        return Err("0 で割った".to_string());
                    }
                    value /= divisor;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn factor(&mut self) -> Result<f64, String> {
        match self.next() {
            Some(Token::Number(value)) => Ok(value),
            Some(Token::Minus) => Ok(-self.factor()?),
            Some(Token::LParen) => {
                let value = self.expr()?;
                match self.next() {
                    Some(Token::RParen) => Ok(value),
                    _ => Err("閉じ括弧がない".to_string()),
                }
            }
            other => Err(format!("予期しない字句: {other:?}")),
        }
    }
}

fn evaluate(input: &str) -> Result<f64, String> {
    let mut parser = Parser { tokens: tokenize(input)?, pos: 0 };
    let value = parser.expr()?;
    if parser.pos != parser.tokens.len() {
        return Err("式の後に余分なものがある".to_string());
    }
    Ok(value)
}

fn main() {
    for input in ["1 + 2 * 3", "(1 + 2) * 3", "-4 / (2 - 4)", "1 / 0"] {
        match evaluate(input) {
            Ok(value) => println!("{input} = {value}"),
            Err(message) => println!("{input}: {message}"),
        }
    }
}
```

## TypeScript

```ts:fizzbuzz.ts
export function fizzbuzz(n: number): string {
  if (n % 15 === 0) return "FizzBuzz";
  if (n % 3 === 0) return "Fizz";
  if (n % 5 === 0) return "Buzz";
  return String(n);
}

for (let n = 1; n <= 15; n++) console.log(fizzbuzz(n));
```

```ts:search.ts
/** 整列した配列から target を探し、見つかればその位置を、なければ -1 を返す。 */
export function binarySearch(items: readonly number[], target: number): number {
  let low = 0;
  let high = items.length;
  while (low < high) {
    const mid = low + Math.floor((high - low) / 2);
    if (items[mid] < target) low = mid + 1;
    else if (items[mid] > target) high = mid;
    else return mid;
  }
  return -1;
}
```

```ts:count.ts
export function countWords(text: string): Map<string, number> {
  const counts = new Map<string, number>();
  for (const word of text.split(/\s+/).filter(Boolean)) {
    counts.set(word, (counts.get(word) ?? 0) + 1);
  }
  return counts;
}

type Entry = { word: string; count: number };

export function topWords(text: string, limit: number): Entry[] {
  return [...countWords(text)]
    .map(([word, count]) => ({ word, count }))
    .sort((a, b) => b.count - a.count || a.word.localeCompare(b.word))
    .slice(0, limit);
}
```

## TSX

```tsx:WordList.tsx
import { useMemo, useState } from "react";
import { topWords } from "./count";

export function WordList({ text }: { text: string }) {
  const [limit, setLimit] = useState(10);
  const words = useMemo(() => topWords(text, limit), [text, limit]);
  return (
    <section>
      <label>
        表示する数
        <input type="number" min={1} value={limit} onChange={(e) => setLimit(Number(e.currentTarget.value))} />
      </label>
      <ol>
        {words.map(({ word, count }) => (
          <li key={word}>
            {word}（{count} 回）
          </li>
        ))}
      </ol>
    </section>
  );
}
```

## JavaScript

```js:fizzbuzz.js
const fizzbuzz = (n) =>
  (n % 3 === 0 ? "Fizz" : "") + (n % 5 === 0 ? "Buzz" : "") || String(n);

console.log(Array.from({ length: 15 }, (_, i) => fizzbuzz(i + 1)).join("\n"));
```

```js:search.js
export function binarySearch(items, target, compare = (a, b) => a - b) {
  let low = 0;
  let high = items.length - 1;
  while (low <= high) {
    const mid = (low + high) >>> 1;
    const order = compare(items[mid], target);
    if (order === 0) return mid;
    if (order < 0) low = mid + 1;
    else high = mid - 1;
  }
  return -1;
}
```

```js:count.js
export const countWords = (text) =>
  text
    .split(/\s+/)
    .filter((word) => word !== "")
    .reduce((counts, word) => ({ ...counts, [word]: (counts[word] ?? 0) + 1 }), {});
```

## Python

```python:fizzbuzz.py
def fizzbuzz(n: int) -> str:
    return "Fizz" * (n % 3 == 0) + "Buzz" * (n % 5 == 0) or str(n)


if __name__ == "__main__":
    for n in range(1, 16):
        print(fizzbuzz(n))
```

```python:search.py
from typing import Sequence


def binary_search(items: Sequence[int], target: int) -> int | None:
    """整列した items から target を探し、見つかればその位置を返す。"""
    low, high = 0, len(items)
    while low < high:
        mid = (low + high) // 2
        if items[mid] < target:
            low = mid + 1
        elif items[mid] > target:
            high = mid
        else:
            return mid
    return None
```

```python:count.py
from collections import Counter


def count_words(text: str) -> Counter[str]:
    return Counter(text.split())


if __name__ == "__main__":
    for word, count in count_words("the cat and the hat").most_common():
        print(f"{word}: {count}")
```

## Haskell

```haskell:FizzBuzz.hs
fizzbuzz :: Int -> String
fizzbuzz n
  | n `mod` 15 == 0 = "FizzBuzz"
  | n `mod` 3 == 0 = "Fizz"
  | n `mod` 5 == 0 = "Buzz"
  | otherwise = show n

main :: IO ()
main = mapM_ (putStrLn . fizzbuzz) [1 .. 15]
```

```haskell:Search.hs
import Data.Array

-- | 整列した配列から値を探し、見つかればその位置を返す。
binarySearch :: Array Int Int -> Int -> Maybe Int
binarySearch items target = go lo (hi + 1)
  where
    (lo, hi) = bounds items
    go low high
      | low >= high = Nothing
      | otherwise =
          let mid = (low + high) `div` 2
           in case compare (items ! mid) target of
                LT -> go (mid + 1) high
                GT -> go low mid
                EQ -> Just mid
```

```haskell:Count.hs
import qualified Data.Map.Strict as Map

countWords :: String -> Map.Map String Int
countWords = Map.fromListWith (+) . map (\word -> (word, 1)) . words

main :: IO ()
main = mapM_ print (Map.toList (countWords "the cat and the hat"))
```

## シェルスクリプト

```sh:fizzbuzz.sh
#!/bin/sh
n=1
while [ "$n" -le 15 ]; do
  if [ $((n % 15)) -eq 0 ]; then
    echo FizzBuzz
  elif [ $((n % 3)) -eq 0 ]; then
    echo Fizz
  elif [ $((n % 5)) -eq 0 ]; then
    echo Buzz
  else
    echo "$n"
  fi
  n=$((n + 1))
done
```

```bash:search.sh
#!/usr/bin/env bash
# 整列した数の並びから値を探し、見つかればその位置を出力する。
binary_search() {
  local target=$1
  shift
  local -a items=("$@")
  local low=0 high=${#items[@]}
  while ((low < high)); do
    local mid=$(((low + high) / 2))
    if ((items[mid] < target)); then
      low=$((mid + 1))
    elif ((items[mid] > target)); then
      high=$mid
    else
      echo "$mid"
      return 0
    fi
  done
  return 1
}

binary_search 7 1 3 5 7 9 11
```

```sh:count.sh
#!/bin/sh
# 語ごとの出現回数を、多い順に出力する。
tr -s '[:space:]' '\n' < "${1:-/dev/stdin}" | sort | uniq -c | sort -rn
```

## SQL

```sql:fizzbuzz.sql
WITH RECURSIVE numbers(n) AS (
  SELECT 1
  UNION ALL
  SELECT n + 1 FROM numbers WHERE n < 15
)
SELECT
  CASE
    WHEN n % 15 = 0 THEN 'FizzBuzz'
    WHEN n % 3 = 0 THEN 'Fizz'
    WHEN n % 5 = 0 THEN 'Buzz'
    ELSE CAST(n AS TEXT)
  END AS value
FROM numbers;
```

```sql:count.sql
-- words(word) に 1 語ずつ入っているとき、語ごとの出現回数を多い順に数える。
SELECT word, COUNT(*) AS count
FROM words
GROUP BY word
ORDER BY count DESC, word
LIMIT 10;
```

```sql:schema.sql
CREATE TABLE articles (
  slug         TEXT PRIMARY KEY,
  title        TEXT NOT NULL,
  published_at TEXT NOT NULL,
  updated_at   TEXT,
  description  TEXT,
  body_md      TEXT NOT NULL
);

SELECT slug, title, COALESCE(updated_at, published_at) AS date
FROM articles
WHERE published_at >= '2026-01-01'
ORDER BY date DESC;
```

## HTML

```html:index.html
<!doctype html>
<html lang="ja">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>語の数え上げ</title>
    <link rel="stylesheet" href="style.css" />
  </head>
  <body>
    <main>
      <h1>語の数え上げ</h1>
      <form id="form">
        <label for="text">文章</label>
        <textarea id="text" rows="8"></textarea>
        <button type="submit">数える</button>
      </form>
      <ol id="result" aria-live="polite"></ol>
    </main>
    <script type="module" src="main.js"></script>
  </body>
</html>
```

## CSS

```css:style.css
:root {
  --text: #000000;
  --link: #0000ee;
  --border: #999999;
}

body {
  margin: 0 auto;
  max-width: 42rem;
  padding: 2rem 1rem;
  color: var(--text);
  font-family: sans-serif;
  line-height: 1.8;
}

a {
  color: var(--link);
  text-decoration: underline;
}

textarea {
  display: block;
  width: 100%;
  border: 1px solid var(--border);
}

@media (prefers-reduced-motion: reduce) {
  * {
    animation: none !important;
  }
}
```

## JSON

```json:package.json
{
  "name": "word-count",
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "test": "node --test"
  },
  "files": ["count.js", "search.js"],
  "engines": {
    "node": ">=22"
  }
}
```

## YAML

```yaml:.github/workflows/test.yml
name: test
on:
  push:
    branches: [main]
  pull_request:
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
      - run: npm ci
      - run: npm test
```

## TOML

```toml:site.toml
title = "語の数え上げの記録"
author = "tett23"
description = "いろいろな言語で同じ処理を書いて比べる。"
url = "https://example.com/"

[license]
name = "CC0 1.0"
url = "https://creativecommons.org/publicdomain/zero/1.0/"

[deploy]
worker = "word-count"
```

## Markdown

````markdown:README.md
# 語の数え上げ

文章を空白で区切り、語ごとの出現回数を数える。

## 使い方

```sh
./count.sh < input.txt
```

| 言語 | ファイル |
|---|---|
| Rust | `count.rs` |
| Python | `count.py` |
````

## diff

言語を指定しない diff と、言語を指定した diff です。

```diff
--- a/count.py
+++ b/count.py
@@ -1,5 +1,5 @@
 from collections import Counter
 
 
-def count_words(text):
+def count_words(text: str) -> Counter[str]:
     return Counter(text.split())
```

```diff rust:search.rs
@@ -1,8 +1,8 @@
 fn binary_search(items: &[i64], target: i64) -> Option<usize> {
-    let (mut low, mut high) = (0, items.len() - 1);
-    while low <= high {
-        let mid = (low + high) / 2;
+    let (mut low, mut high) = (0, items.len());
+    while low < high {
+        let mid = low + (high - low) / 2;
         match items[mid].cmp(&target) {
             std::cmp::Ordering::Less => low = mid + 1,
-            std::cmp::Ordering::Greater => high = mid - 1,
+            std::cmp::Ordering::Greater => high = mid,
```

```diff ts:count.ts
@@ -1,6 +1,6 @@
 export function countWords(text: string): Map<string, number> {
   const counts = new Map<string, number>();
-  for (const word of text.split(" ")) {
+  for (const word of text.split(/\s+/).filter(Boolean)) {
     counts.set(word, (counts.get(word) ?? 0) + 1);
   }
   return counts;
```

## 色を付けない言語

言語名のないコードブロックと、登録していない言語のコードブロックは、色なしで表示します。

```
$ ./count.sh < input.txt
      2 the
      1 hat
      1 cat
      1 and
```

```ruby
def fizzbuzz(n)
  return "FizzBuzz" if (n % 15).zero?
  return "Fizz" if (n % 3).zero?
  return "Buzz" if (n % 5).zero?
  n.to_s
end
```
