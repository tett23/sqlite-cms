---
title: "複雑なページの見本：入れ子の深い記事"
date: 2026-09-26
description: 表示の重さを確かめるための見本。リスト、引用、折りたたみ、メッセージを深く入れ子にし、表の中に数式やコードを置き、HTML のタグと脚注を多く混ぜる。
tags: [見本, 計測]
---

:::message
このページは、表示の重さを確かめるための見本です。
量は多くありませんが、要素の入れ子が深く、一つの要素に多くの種類の記法が混ざっています。
Markdown の解析と、本文の HTML の取り込み（rehype-raw）と無害化（rehype-sanitize）の重さを計測しています。
:::

## 深いリスト

sqlite-cms で記事を公開するまでの手順を、細かく分けて書きます[^steps]。

[^steps]: 実際には `sqlite-cms deploy` 一つで済む。

1. 記事を書く
   - 書く場所を決める
     - post（日々のメモ）
       - `content/posts/` に置く
       - 日付は `new` が入れる
         - タイムゾーンは `site.toml` の `timezone`^[省略すると、実行した環境のタイムゾーン。]
         - 書き換えてもよい
     - article（長い読み物）
       - `content/articles/` に置く
       - `description` を書く
         - 一覧と `meta description` に使う
         - RSS の説明にも使う^[書かないときは本文の書き出し。]
   - 本文を書く
     - [x] 見出しを立てる
     - [x] 段落を分ける
     - [ ] コードに言語名を付ける
       - [x] 色を付ける言語
       - [ ] 色を付けない言語
         - [ ] ファイル名だけを付ける
2. 確かめる
   1. `sqlite-cms serve` を起動する
      1. 保存すると、ブラウザが読み込み直す
      2. 読み込み直さないときは
         1. `--no-reload` を付けていないか
         2. ブラウザを再読み込みする
   2. Lighthouse で計測する
      - ユーザー補助とおすすめの方法は **100 点**
      - パフォーマンスは記録するだけ
3. 公開する
   - Cloudflare
     - `sqlite-cms deploy`
   - GitHub Pages と rsync
     - `sqlite-cms build` で書き出して、置く

定義のリストも入れ子にします。

<dl>
<dt>post</dt>
<dd>日々の雑多なメモ。<kbd>Ctrl</kbd> + <kbd>S</kbd> で保存して、すぐに公開する。
<dl>
<dt>置き場所</dt>
<dd><code>content/posts/</code></dd>
<dt>日付</dt>
<dd>必須。<mark>公開日だけ</mark>を持つ。</dd>
</dl>
</dd>
<dt>article</dt>
<dd>長めの読み物。改稿日と要約を持つ。
<dl>
<dt>置き場所</dt>
<dd><code>content/articles/</code></dd>
<dt>改稿日</dt>
<dd>任意。<abbr title="Really Simple Syndication">RSS</abbr> には公開日を載せる。</dd>
</dl>
</dd>
</dl>

## 深い引用

> 引用の中に、引用を書けます。
>
> > 二段目の引用です。
> > リストも書けます。
> >
> > - 一つめ
> > - 二つめ
> >   > 三段目の引用です。
> >   > `コード`と **強調** と [記法の一覧](/articles/syntax) へのリンクを含みます。
> >   >
> >   > ```ts
> >   > const depth = 3;
> >   > ```
>
> 一段目に戻ります。$E = mc^2$ のような数式も書けます。

## 折りたたみとメッセージの入れ子

:::::details 一段目の折りたたみ
一段目です。

::::details 二段目の折りたたみ
二段目です。

:::message
二段目の中のメッセージです。

- リストと
- `コード`と
- 数式 $\sum_{k=1}^{n} k = \frac{n(n+1)}{2}$ を含みます。

```rust:depth.rs
fn depth(level: u32) -> String {
    "  ".repeat(level as usize) + "ここ"
}
```
:::

:::message alert
二段目の中の警告です。

| 段 | 要素 |
|---|---|
| 1 | details |
| 2 | details |
| 3 | message |
:::
::::

一段目に戻ります。
:::::

## 表の中のいろいろな記法

| 記法 | 書き方 | 表示 | 注 |
|---|---|---|---|
| 強調 | `**強調**` | **強調** | 太字にする[^bold] |
| コード | `` `code` `` | `code` | 等幅にする |
| リンク | `[記法](/articles/syntax)` | [記法](/articles/syntax) | サイトの中は移動が速い |
| 数式 | `$a^2 + b^2 = c^2$` | $a^2 + b^2 = c^2$ | KaTeX で描く |
| 分数 | `$\frac{1}{2}$` | $\frac{1}{2}$ | 表の中でも描ける |
| ルビ | `<ruby>` | <ruby>漢字<rt>かんじ</rt></ruby> | HTML で書く |
| キー | `<kbd>` | <kbd>Esc</kbd> | HTML で書く |
| 上付き | `<sup>` | x<sup>2</sup> | HTML で書く |
| 下付き | `<sub>` | H<sub>2</sub>O | HTML で書く |
| 印 | `<mark>` | <mark>印</mark> | HTML で書く |
| 取り消し | `~~消す~~` | ~~消す~~ | GFM |
| 画像 | `![](/media/sample.svg =40x)` | ![古いウェブサイトのようなページの小さな絵](/media/sample.svg =40x) | 幅を指定できる |

[^bold]: 日本語の斜体は読みにくいので、強調には太字を使う。

右寄せと中央寄せの列も混ぜます。

| 左 | 中央 | 右 | 数式 |
|:---|:---:|---:|:---:|
| 1 | 一 | 1,000 | $10^3$ |
| 2 | 二 | 1,000,000 | $10^6$ |
| 3 | 三 | 1,000,000,000 | $10^9$ |

## リストの中のコードと数式

1. 式を立てる

   $$
   f(x) = \int_0^x e^{-t^2}\,dt
   $$

2. コードにする

   ```python:erf.py
   import math


   def f(x: float, steps: int = 1000) -> float:
       """0 から x までの e^{-t^2} を台形公式で積分する。"""
       h = x / steps
       total = (1 + math.exp(-(x**2))) / 2
       for i in range(1, steps):
           total += math.exp(-((i * h) ** 2))
       return total * h
   ```

3. 確かめる

   - $\displaystyle \lim_{x \to \infty} f(x) = \frac{\sqrt{\pi}}{2}$
   - 差分は次のとおり

     ```diff python:erf.py
     -    h = x / steps
     +    h = x / max(steps, 1)
     ```

## HTML の中の Markdown

<details>
<summary><strong>HTML で書いた折りたたみ</strong>と<code>コード</code></summary>

HTML の要素の中でも、空行を挟めば Markdown が書けます。

- リスト
- **強調**
- $x^2$

</details>

<details open>
<summary>最初から開いている折りたたみ</summary>

| 列 | 値 |
|---|---|
| open | 最初から開く |

</details>

## 脚注の多い段落

組版[^1]は、字[^2]を並べて[^3]、行[^4]と段落[^5]を作り、ページ[^6]に置く作業です。
画面[^7]では、ページの大きさ[^8]が決まっていないので、行の長さ[^9]は画面の幅[^10]で変わります。

[^1]: 版を組むこと。
[^2]: 漢字、かな、英数字、約物。
[^3]: 字の間の空きも決める。
[^4]: 一行の字数と行間。
[^5]: 字下げと段落の間の空き。
[^6]: 版面と余白。
[^7]: ブラウザで読むとき。
[^8]: 紙の大きさに当たるもの。
[^9]: このサイトでは本文の幅の上限を決めている。
[^10]: 端末によって大きく違う。
