---
title: "記法の一覧"
date: 2026-09-25
description: Markdown と GFM の記法、コードの色分け、本文に書ける HTML のタグを、実際の表示と一緒に並べたもの。
---

本文は Markdown で書きます。
[CommonMark](https://commonmark.org/) に、GitHub の拡張（GFM）を加えた記法が使えます。

## 文字の装飾

**強調**、*斜体*、~~取り消し線~~、`行内のコード`。
URL は www.example.com のように書くだけでリンクになり、サイト内へのリンクは [自己紹介](/about) のように書きます。

## リスト

- 箇条書き
  - 入れ子にもできる
- 二つ目

1. 番号付き
2. 二つ目

- [x] 終わったこと
- [ ] まだのこと

## 引用

> 引用はこう表示されます。
> 複数行にわたっても一つの引用です。

## 表

| 記法 | 書き方 | 備考 |
|---|---|---|
| 強調 | `**強調**` | |
| 取り消し線 | `~~取り消し線~~` | GFM |
| 改行 | `<br>` | 表のセルの中<br>だけで使う |

## コード

言語名を書くと色が付きます。
対応する言語は README の一覧を参照してください。

```haskell
fizzBuzz :: Int -> String
fizzBuzz n
  | n `mod` 15 == 0 = "FizzBuzz"
  | n `mod` 3 == 0 = "Fizz"
  | n `mod` 5 == 0 = "Buzz"
  | otherwise = show n
```

言語名を書かないと、色なしで表示されます。

```
色なしのブロック
```

## 脚注

脚注は本文の最後にまとまります[^footnote]。

[^footnote]: 脚注の本文です。番号を押すと本文に戻れます。

## 画像

画像は `content/media/` に置き、`/media/` から参照します。

![sample image と書かれた四角](/media/sample.svg)

## HTML で書くもの

Markdown では書けない表現は、決まった HTML のタグで書けます。

<dl>
<dt>キー操作</dt>
<dd>保存は <kbd>Ctrl</kbd> + <kbd>S</kbd>。</dd>
<dt>下付きと上付き</dt>
<dd>H<sub>2</sub>O、E = mc<sup>2</sup>。</dd>
<dt>ルビ</dt>
<dd><ruby>組版<rp>（</rp><rt>くみはん</rt><rp>）</rp></ruby>、<ruby>禁則<rp>（</rp><rt>きんそく</rt><rp>）</rp></ruby>。</dd>
<dt>蛍光ペン</dt>
<dd><mark>ここが大事</mark>。</dd>
<dt>略語</dt>
<dd><abbr title="Content Management System">CMS</abbr> にマウスを乗せると正式名が出ます。</dd>
</dl>

<details>
<summary>折りたたみ（<code>details</code>）</summary>

中には Markdown が書けます。
`<summary>` の後と `</details>` の前に空行を入れるのを忘れずに。

```html
<details>
<summary>見出し</summary>

中身

</details>
```

</details>

`div` や `span` などほかのタグはタグだけが外れ、`<script>` は中身ごと消えます。

## 拡張の記法

メッセージ、数式、図などの拡張は [拡張の記法](/articles/extensions) にまとめています。
