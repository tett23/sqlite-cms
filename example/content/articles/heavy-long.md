---
title: "重いページの見本：長い記事"
date: 2026-09-26
description: 表示の重さを確かめるための見本。日本語の文章を組むときの約束ごとを、見出し、リスト、表、脚注、引用を使って長く書き、付録に文字コードの大きな表を付ける。
---

:::message
このページは、表示の重さを確かめるための見本です。
本文が長い記事で、Markdown の解析と描画、全文検索の索引づくりが、表示や入力への応答をどれだけ遅らせるかを Lighthouse で計測しています。
:::

日本語の文章を画面や紙に組むときの約束ごとを、思いつくままに書きます。
細かな規則は JIS X 4051（日本語文書の組版方法）や、W3C の「日本語組版処理の要件」（JLReq）にまとまっています[^jlreq]。
ここではそれらを引き写すのではなく、ブログのような短い文章を書くときに気にしていることを並べます。

[^jlreq]: JLReq は日本語と英語で公開されていて、図が多く読みやすい。

## 字

### 全角と半角

日本語の文章では、漢字、かな、全角の約物（句読点や括弧）と、半角の英数字を混ぜて書きます。
英数字を全角で書くと、検索で見つけにくくなり、等幅でない字体では字間も不揃いになります。
このサイトの記事では、英数字はすべて半角で書いています。

- 数字：`2026年9月26日` のように半角で書く
- 英字：`SQLite`、`Markdown` のように半角で書く
- 記号：`%` や `#` は半角、`「」` や `、。` は全角で書く

### 句読点

句読点は、横書きでも「、」と「。」を使います。
「，」と「．」を使う書き方もあり、理科系の論文や公用文の一部で見かけます。
一つの文章の中では、どちらかにそろえます。

### 括弧

かぎ括弧「」は会話や引用、語の強調に使い、二重かぎ括弧『』は書名や、かぎ括弧の中のかぎ括弧に使います。
丸括弧（）は補足に使います。
括弧の中の文の終わりに句点を付けるかどうかは、流儀が分かれます。

> 閉じ括弧の直前の句点は省く。ただし、括弧の中が複数の文からなるときは、最後の文にも句点を付けることがある。

## 字の間

### 和欧間

日本語と英数字の間に空白を入れるかどうかも、流儀が分かれます。
組版のソフトウェアは、和字と欧字の間に四分の一の空き（四分アキ）を自動で入れるものが多く、そのときは原稿に空白を書きません。
ブラウザは自動では空けないので、原稿に半角の空白を書く人もいます。

| 書き方 | 例 | 利点 | 欠点 |
|---|---|---|---|
| 空白を書かない | SQLiteのファイル | 原稿が素直 | ブラウザでは詰まって見える |
| 半角の空白を書く | SQLite のファイル | ブラウザで読みやすい | 空きが広すぎることがある |
| 組版で空ける | SQLite␣のファイル | 空きの量を調整できる | 対応したソフトウェアが要る |

このサイトの記事では、半角の空白を書いています。

### 約物の空き

全角の約物は、字の形の半分ほどが空白です。
「」や（）が続くと、空白が重なって間延びして見えるので、組版では半分を詰めます（約物の連続）。
CSS では `text-spacing-trim` で指定できるようになりつつあります[^trim]。

[^trim]: 対応しているブラウザはまだ少ない。

## 行

### 行の長さ

一行の字数が多すぎると、行末から次の行頭へ目を移すのが難しくなります。
横書きの本文では、一行 35 字から 40 字くらいが読みやすいと言われます。
このサイトは本文の幅を `42rem` に制限していて、文字の大きさが 16 px なら一行はおよそ 42 字です。

### 行間

日本語は字の形が正方形に近く、行間が狭いと行と行がくっついて見えます。
行の高さを字の大きさの 1.7 倍から 2 倍くらいにすると、読みやすくなります。

### 禁則

行頭に句読点や閉じ括弧が来ないようにし、行末に開き括弧が来ないようにします。
これを禁則処理と呼びます。

- 行頭に置かない：`、。」）』！？ー` や小書きのかな（`ゃゅょっ`）
- 行末に置かない：`「（『`
- 分けない：`……` や `——` のような二つで一組の記号、数字と単位

ブラウザは主な禁則を処理しますが、小書きのかなや長音記号を行頭に置かない（厳しい禁則）かどうかは、CSS の `line-break` で選べます。

::::details 禁則の強さの違い
:::message
`line-break: strict` は、小書きのかなや長音記号も行頭に置かない。
`line-break: normal` は、それらを行頭に置くことを許す。
:::

新聞や雑誌は、字詰めを保つために弱い禁則を使うことが多いと言われます。
::::

## 段落

### 字下げ

紙の本では、段落の最初の字を一字下げます。
ウェブでは段落の間を一行空けて区切ることが多く、字下げはしないことが多いです。
両方をすると、区切りが重なってしまいます。

### 段落の長さ

画面で読む文章は、紙よりも短い段落が好まれます。
一つの段落には一つの話題だけを書き、三文から五文くらいで区切ると、読み手が迷いにくくなります。

## 見出し

見出しは、文章の構造を示すものです。
字を大きくするためだけに見出しを使うと、読み上げのソフトウェアや目次が構造を読み違えます。

1. 記事の題名は `h1` にする（このサイトでは記事の題名が自動で `h1` になる）
2. 本文の大見出しは `h2`、その下は `h3` にする
3. 段を飛ばさない（`h2` の次に `h4` を置かない）

## 強調

強調には太字（`**太字**`）を使います。
日本語の斜体は、字の形を傾けるだけのものが多く、読みにくくなります。
圏点（傍点）は、CSS の `text-emphasis` で付けられます。

強調が多いと、どれも目立たなくなります。
**一つの段落に一か所まで**、くらいが目安です。

## ルビ

読みにくい語には、ルビ（振り仮名）を振ります。
HTML では <ruby>組版<rp>（</rp><rt>くみはん</rt><rp>）</rp></ruby> のように `ruby` と `rt` で書きます。
`rp` の括弧は、ルビに対応していない環境で表示されます。

ルビの振り方には、語全体に振るグループルビと、一字ずつ振るモノルビがあります。

| 振り方 | 例 | 向いている語 |
|---|---|---|
| グループルビ | <ruby>明日<rt>あした</rt></ruby> | 熟字訓（一字ずつに読みを分けられない語） |
| モノルビ | <ruby>東<rt>とう</rt>京<rt>きょう</rt></ruby> | 一字ずつに読みを分けられる語 |

## 数字

### 桁区切り

大きな数は、三桁ごとにコンマで区切ります（`1,234,567`）。
「万」「億」を使う書き方（`123万4,567`）もあり、日本語の文章ではこちらが読みやすいこともあります。

### 単位

数字と単位の間に空白を入れるかどうかは、単位によって変えることがあります。
SI 単位では、数値と単位記号の間に空白を入れます（`16 px`、`42 KB`）。
ただし `%` や `°` は、空白を入れずに書くことが多いです。

## 記号

### ダッシュとリーダー

- 全角ダッシュ（`—`）は二つ続けて `——` と書き、語句の言い換えや補足に使う
- 三点リーダー（`…`）も二つ続けて `……` と書き、言いさしや省略に使う
- 波ダッシュ（`〜`）は範囲を表す（`9月26日〜30日`）

### 引用符

横書きでも、日本語の文章の中では「」を使います。
英語の引用符 “ ” は、英文を引くときに使います。
ダブルミニュート〝 〟を使う書き方もあります。

## 文体

### です・ます と だ・である

一つの文章の中では、文末をどちらかにそろえます。
このサイトの記事は「です・ます」で、ブログは「だ・である」で書いています。
ただし、箇条書きの項目や表の中は、体言止めや「〜する」で終えることがあります。

### 漢字とかな

漢字が多いと文章が硬く見え、かなが多いと語の切れ目が分かりにくくなります。
次のような語は、かなで書くと読みやすくなります（公用文の書き方にならっています）。

| 漢字で書くと | かなで書くと |
|---|---|
| 出来る | できる |
| 事 | こと |
| 時（形式名詞として） | とき |
| 様に | ように |
| 下さい | ください |
| 及び | および |
| 但し | ただし |
| 即ち | すなわち |
| 殆ど | ほとんど |
| 予め | あらかじめ |

## 画面で読む文章

### リンク

リンクの文字には、行き先が分かる語を選びます。
「こちら」や「ここ」だけをリンクにすると、リンクだけを拾い読みしたときに意味が分かりません。
このサイトのリンクは、青い字に下線を引いています。
色だけで区別すると、色を見分けにくい人には分かりません。

### 画像

画像には、画像の内容を伝える代替テキストを付けます。
飾りのための画像には、空の代替テキスト（`alt=""`）を付けて、読み上げで飛ばされるようにします。

### コントラスト

文字の色と背景の色のコントラストは、4.5:1 以上にします（WCAG の AA）。
このサイトは白地に黒い文字なので、21:1 です。
コードの色分けも、すべての色が背景に対して 4.5:1 以上になるテーマを選んでいます。

## まとめ

約束ごとの多くは、読み手が迷わないためのものです。
迷ったときは、次のことを確かめます。

- [x] 一つの文章の中で、書き方がそろっているか
- [x] 見出しが構造を表しているか
- [x] リンクと画像が、見えなくても伝わるか
- [ ] 声に出して読んで、つかえないか

次の付録は、文字コードの表です。
表示の重さを確かめるために、大きな表を載せています。

## 付録：文字コードの表

文字、コードポイント、Unicode の文字名、UTF-8 のバイト列を並べます。
表は Python の `unicodedata` で作りました。

### ひらがな

ひらがなの区画（U+3040〜U+309F）の文字です。（93 字）

| 文字 | コードポイント | 文字名 | UTF-8 |
|---|---|---|---|
| ぁ | U+3041 | HIRAGANA LETTER SMALL A | `E3 81 81` |
| あ | U+3042 | HIRAGANA LETTER A | `E3 81 82` |
| ぃ | U+3043 | HIRAGANA LETTER SMALL I | `E3 81 83` |
| い | U+3044 | HIRAGANA LETTER I | `E3 81 84` |
| ぅ | U+3045 | HIRAGANA LETTER SMALL U | `E3 81 85` |
| う | U+3046 | HIRAGANA LETTER U | `E3 81 86` |
| ぇ | U+3047 | HIRAGANA LETTER SMALL E | `E3 81 87` |
| え | U+3048 | HIRAGANA LETTER E | `E3 81 88` |
| ぉ | U+3049 | HIRAGANA LETTER SMALL O | `E3 81 89` |
| お | U+304A | HIRAGANA LETTER O | `E3 81 8A` |
| か | U+304B | HIRAGANA LETTER KA | `E3 81 8B` |
| が | U+304C | HIRAGANA LETTER GA | `E3 81 8C` |
| き | U+304D | HIRAGANA LETTER KI | `E3 81 8D` |
| ぎ | U+304E | HIRAGANA LETTER GI | `E3 81 8E` |
| く | U+304F | HIRAGANA LETTER KU | `E3 81 8F` |
| ぐ | U+3050 | HIRAGANA LETTER GU | `E3 81 90` |
| け | U+3051 | HIRAGANA LETTER KE | `E3 81 91` |
| げ | U+3052 | HIRAGANA LETTER GE | `E3 81 92` |
| こ | U+3053 | HIRAGANA LETTER KO | `E3 81 93` |
| ご | U+3054 | HIRAGANA LETTER GO | `E3 81 94` |
| さ | U+3055 | HIRAGANA LETTER SA | `E3 81 95` |
| ざ | U+3056 | HIRAGANA LETTER ZA | `E3 81 96` |
| し | U+3057 | HIRAGANA LETTER SI | `E3 81 97` |
| じ | U+3058 | HIRAGANA LETTER ZI | `E3 81 98` |
| す | U+3059 | HIRAGANA LETTER SU | `E3 81 99` |
| ず | U+305A | HIRAGANA LETTER ZU | `E3 81 9A` |
| せ | U+305B | HIRAGANA LETTER SE | `E3 81 9B` |
| ぜ | U+305C | HIRAGANA LETTER ZE | `E3 81 9C` |
| そ | U+305D | HIRAGANA LETTER SO | `E3 81 9D` |
| ぞ | U+305E | HIRAGANA LETTER ZO | `E3 81 9E` |
| た | U+305F | HIRAGANA LETTER TA | `E3 81 9F` |
| だ | U+3060 | HIRAGANA LETTER DA | `E3 81 A0` |
| ち | U+3061 | HIRAGANA LETTER TI | `E3 81 A1` |
| ぢ | U+3062 | HIRAGANA LETTER DI | `E3 81 A2` |
| っ | U+3063 | HIRAGANA LETTER SMALL TU | `E3 81 A3` |
| つ | U+3064 | HIRAGANA LETTER TU | `E3 81 A4` |
| づ | U+3065 | HIRAGANA LETTER DU | `E3 81 A5` |
| て | U+3066 | HIRAGANA LETTER TE | `E3 81 A6` |
| で | U+3067 | HIRAGANA LETTER DE | `E3 81 A7` |
| と | U+3068 | HIRAGANA LETTER TO | `E3 81 A8` |
| ど | U+3069 | HIRAGANA LETTER DO | `E3 81 A9` |
| な | U+306A | HIRAGANA LETTER NA | `E3 81 AA` |
| に | U+306B | HIRAGANA LETTER NI | `E3 81 AB` |
| ぬ | U+306C | HIRAGANA LETTER NU | `E3 81 AC` |
| ね | U+306D | HIRAGANA LETTER NE | `E3 81 AD` |
| の | U+306E | HIRAGANA LETTER NO | `E3 81 AE` |
| は | U+306F | HIRAGANA LETTER HA | `E3 81 AF` |
| ば | U+3070 | HIRAGANA LETTER BA | `E3 81 B0` |
| ぱ | U+3071 | HIRAGANA LETTER PA | `E3 81 B1` |
| ひ | U+3072 | HIRAGANA LETTER HI | `E3 81 B2` |
| び | U+3073 | HIRAGANA LETTER BI | `E3 81 B3` |
| ぴ | U+3074 | HIRAGANA LETTER PI | `E3 81 B4` |
| ふ | U+3075 | HIRAGANA LETTER HU | `E3 81 B5` |
| ぶ | U+3076 | HIRAGANA LETTER BU | `E3 81 B6` |
| ぷ | U+3077 | HIRAGANA LETTER PU | `E3 81 B7` |
| へ | U+3078 | HIRAGANA LETTER HE | `E3 81 B8` |
| べ | U+3079 | HIRAGANA LETTER BE | `E3 81 B9` |
| ぺ | U+307A | HIRAGANA LETTER PE | `E3 81 BA` |
| ほ | U+307B | HIRAGANA LETTER HO | `E3 81 BB` |
| ぼ | U+307C | HIRAGANA LETTER BO | `E3 81 BC` |
| ぽ | U+307D | HIRAGANA LETTER PO | `E3 81 BD` |
| ま | U+307E | HIRAGANA LETTER MA | `E3 81 BE` |
| み | U+307F | HIRAGANA LETTER MI | `E3 81 BF` |
| む | U+3080 | HIRAGANA LETTER MU | `E3 82 80` |
| め | U+3081 | HIRAGANA LETTER ME | `E3 82 81` |
| も | U+3082 | HIRAGANA LETTER MO | `E3 82 82` |
| ゃ | U+3083 | HIRAGANA LETTER SMALL YA | `E3 82 83` |
| や | U+3084 | HIRAGANA LETTER YA | `E3 82 84` |
| ゅ | U+3085 | HIRAGANA LETTER SMALL YU | `E3 82 85` |
| ゆ | U+3086 | HIRAGANA LETTER YU | `E3 82 86` |
| ょ | U+3087 | HIRAGANA LETTER SMALL YO | `E3 82 87` |
| よ | U+3088 | HIRAGANA LETTER YO | `E3 82 88` |
| ら | U+3089 | HIRAGANA LETTER RA | `E3 82 89` |
| り | U+308A | HIRAGANA LETTER RI | `E3 82 8A` |
| る | U+308B | HIRAGANA LETTER RU | `E3 82 8B` |
| れ | U+308C | HIRAGANA LETTER RE | `E3 82 8C` |
| ろ | U+308D | HIRAGANA LETTER RO | `E3 82 8D` |
| ゎ | U+308E | HIRAGANA LETTER SMALL WA | `E3 82 8E` |
| わ | U+308F | HIRAGANA LETTER WA | `E3 82 8F` |
| ゐ | U+3090 | HIRAGANA LETTER WI | `E3 82 90` |
| ゑ | U+3091 | HIRAGANA LETTER WE | `E3 82 91` |
| を | U+3092 | HIRAGANA LETTER WO | `E3 82 92` |
| ん | U+3093 | HIRAGANA LETTER N | `E3 82 93` |
| ゔ | U+3094 | HIRAGANA LETTER VU | `E3 82 94` |
| ゕ | U+3095 | HIRAGANA LETTER SMALL KA | `E3 82 95` |
| ゖ | U+3096 | HIRAGANA LETTER SMALL KE | `E3 82 96` |
| ゙ | U+3099 | COMBINING KATAKANA-HIRAGANA VOICED SOUND MARK | `E3 82 99` |
| ゚ | U+309A | COMBINING KATAKANA-HIRAGANA SEMI-VOICED SOUND MARK | `E3 82 9A` |
| ゛ | U+309B | KATAKANA-HIRAGANA VOICED SOUND MARK | `E3 82 9B` |
| ゜ | U+309C | KATAKANA-HIRAGANA SEMI-VOICED SOUND MARK | `E3 82 9C` |
| ゝ | U+309D | HIRAGANA ITERATION MARK | `E3 82 9D` |
| ゞ | U+309E | HIRAGANA VOICED ITERATION MARK | `E3 82 9E` |
| ゟ | U+309F | HIRAGANA DIGRAPH YORI | `E3 82 9F` |

### カタカナ

カタカナの区画（U+30A0〜U+30FF）の文字です。（96 字）

| 文字 | コードポイント | 文字名 | UTF-8 |
|---|---|---|---|
| ゠ | U+30A0 | KATAKANA-HIRAGANA DOUBLE HYPHEN | `E3 82 A0` |
| ァ | U+30A1 | KATAKANA LETTER SMALL A | `E3 82 A1` |
| ア | U+30A2 | KATAKANA LETTER A | `E3 82 A2` |
| ィ | U+30A3 | KATAKANA LETTER SMALL I | `E3 82 A3` |
| イ | U+30A4 | KATAKANA LETTER I | `E3 82 A4` |
| ゥ | U+30A5 | KATAKANA LETTER SMALL U | `E3 82 A5` |
| ウ | U+30A6 | KATAKANA LETTER U | `E3 82 A6` |
| ェ | U+30A7 | KATAKANA LETTER SMALL E | `E3 82 A7` |
| エ | U+30A8 | KATAKANA LETTER E | `E3 82 A8` |
| ォ | U+30A9 | KATAKANA LETTER SMALL O | `E3 82 A9` |
| オ | U+30AA | KATAKANA LETTER O | `E3 82 AA` |
| カ | U+30AB | KATAKANA LETTER KA | `E3 82 AB` |
| ガ | U+30AC | KATAKANA LETTER GA | `E3 82 AC` |
| キ | U+30AD | KATAKANA LETTER KI | `E3 82 AD` |
| ギ | U+30AE | KATAKANA LETTER GI | `E3 82 AE` |
| ク | U+30AF | KATAKANA LETTER KU | `E3 82 AF` |
| グ | U+30B0 | KATAKANA LETTER GU | `E3 82 B0` |
| ケ | U+30B1 | KATAKANA LETTER KE | `E3 82 B1` |
| ゲ | U+30B2 | KATAKANA LETTER GE | `E3 82 B2` |
| コ | U+30B3 | KATAKANA LETTER KO | `E3 82 B3` |
| ゴ | U+30B4 | KATAKANA LETTER GO | `E3 82 B4` |
| サ | U+30B5 | KATAKANA LETTER SA | `E3 82 B5` |
| ザ | U+30B6 | KATAKANA LETTER ZA | `E3 82 B6` |
| シ | U+30B7 | KATAKANA LETTER SI | `E3 82 B7` |
| ジ | U+30B8 | KATAKANA LETTER ZI | `E3 82 B8` |
| ス | U+30B9 | KATAKANA LETTER SU | `E3 82 B9` |
| ズ | U+30BA | KATAKANA LETTER ZU | `E3 82 BA` |
| セ | U+30BB | KATAKANA LETTER SE | `E3 82 BB` |
| ゼ | U+30BC | KATAKANA LETTER ZE | `E3 82 BC` |
| ソ | U+30BD | KATAKANA LETTER SO | `E3 82 BD` |
| ゾ | U+30BE | KATAKANA LETTER ZO | `E3 82 BE` |
| タ | U+30BF | KATAKANA LETTER TA | `E3 82 BF` |
| ダ | U+30C0 | KATAKANA LETTER DA | `E3 83 80` |
| チ | U+30C1 | KATAKANA LETTER TI | `E3 83 81` |
| ヂ | U+30C2 | KATAKANA LETTER DI | `E3 83 82` |
| ッ | U+30C3 | KATAKANA LETTER SMALL TU | `E3 83 83` |
| ツ | U+30C4 | KATAKANA LETTER TU | `E3 83 84` |
| ヅ | U+30C5 | KATAKANA LETTER DU | `E3 83 85` |
| テ | U+30C6 | KATAKANA LETTER TE | `E3 83 86` |
| デ | U+30C7 | KATAKANA LETTER DE | `E3 83 87` |
| ト | U+30C8 | KATAKANA LETTER TO | `E3 83 88` |
| ド | U+30C9 | KATAKANA LETTER DO | `E3 83 89` |
| ナ | U+30CA | KATAKANA LETTER NA | `E3 83 8A` |
| ニ | U+30CB | KATAKANA LETTER NI | `E3 83 8B` |
| ヌ | U+30CC | KATAKANA LETTER NU | `E3 83 8C` |
| ネ | U+30CD | KATAKANA LETTER NE | `E3 83 8D` |
| ノ | U+30CE | KATAKANA LETTER NO | `E3 83 8E` |
| ハ | U+30CF | KATAKANA LETTER HA | `E3 83 8F` |
| バ | U+30D0 | KATAKANA LETTER BA | `E3 83 90` |
| パ | U+30D1 | KATAKANA LETTER PA | `E3 83 91` |
| ヒ | U+30D2 | KATAKANA LETTER HI | `E3 83 92` |
| ビ | U+30D3 | KATAKANA LETTER BI | `E3 83 93` |
| ピ | U+30D4 | KATAKANA LETTER PI | `E3 83 94` |
| フ | U+30D5 | KATAKANA LETTER HU | `E3 83 95` |
| ブ | U+30D6 | KATAKANA LETTER BU | `E3 83 96` |
| プ | U+30D7 | KATAKANA LETTER PU | `E3 83 97` |
| ヘ | U+30D8 | KATAKANA LETTER HE | `E3 83 98` |
| ベ | U+30D9 | KATAKANA LETTER BE | `E3 83 99` |
| ペ | U+30DA | KATAKANA LETTER PE | `E3 83 9A` |
| ホ | U+30DB | KATAKANA LETTER HO | `E3 83 9B` |
| ボ | U+30DC | KATAKANA LETTER BO | `E3 83 9C` |
| ポ | U+30DD | KATAKANA LETTER PO | `E3 83 9D` |
| マ | U+30DE | KATAKANA LETTER MA | `E3 83 9E` |
| ミ | U+30DF | KATAKANA LETTER MI | `E3 83 9F` |
| ム | U+30E0 | KATAKANA LETTER MU | `E3 83 A0` |
| メ | U+30E1 | KATAKANA LETTER ME | `E3 83 A1` |
| モ | U+30E2 | KATAKANA LETTER MO | `E3 83 A2` |
| ャ | U+30E3 | KATAKANA LETTER SMALL YA | `E3 83 A3` |
| ヤ | U+30E4 | KATAKANA LETTER YA | `E3 83 A4` |
| ュ | U+30E5 | KATAKANA LETTER SMALL YU | `E3 83 A5` |
| ユ | U+30E6 | KATAKANA LETTER YU | `E3 83 A6` |
| ョ | U+30E7 | KATAKANA LETTER SMALL YO | `E3 83 A7` |
| ヨ | U+30E8 | KATAKANA LETTER YO | `E3 83 A8` |
| ラ | U+30E9 | KATAKANA LETTER RA | `E3 83 A9` |
| リ | U+30EA | KATAKANA LETTER RI | `E3 83 AA` |
| ル | U+30EB | KATAKANA LETTER RU | `E3 83 AB` |
| レ | U+30EC | KATAKANA LETTER RE | `E3 83 AC` |
| ロ | U+30ED | KATAKANA LETTER RO | `E3 83 AD` |
| ヮ | U+30EE | KATAKANA LETTER SMALL WA | `E3 83 AE` |
| ワ | U+30EF | KATAKANA LETTER WA | `E3 83 AF` |
| ヰ | U+30F0 | KATAKANA LETTER WI | `E3 83 B0` |
| ヱ | U+30F1 | KATAKANA LETTER WE | `E3 83 B1` |
| ヲ | U+30F2 | KATAKANA LETTER WO | `E3 83 B2` |
| ン | U+30F3 | KATAKANA LETTER N | `E3 83 B3` |
| ヴ | U+30F4 | KATAKANA LETTER VU | `E3 83 B4` |
| ヵ | U+30F5 | KATAKANA LETTER SMALL KA | `E3 83 B5` |
| ヶ | U+30F6 | KATAKANA LETTER SMALL KE | `E3 83 B6` |
| ヷ | U+30F7 | KATAKANA LETTER VA | `E3 83 B7` |
| ヸ | U+30F8 | KATAKANA LETTER VI | `E3 83 B8` |
| ヹ | U+30F9 | KATAKANA LETTER VE | `E3 83 B9` |
| ヺ | U+30FA | KATAKANA LETTER VO | `E3 83 BA` |
| ・ | U+30FB | KATAKANA MIDDLE DOT | `E3 83 BB` |
| ー | U+30FC | KATAKANA-HIRAGANA PROLONGED SOUND MARK | `E3 83 BC` |
| ヽ | U+30FD | KATAKANA ITERATION MARK | `E3 83 BD` |
| ヾ | U+30FE | KATAKANA VOICED ITERATION MARK | `E3 83 BE` |
| ヿ | U+30FF | KATAKANA DIGRAPH KOTO | `E3 83 BF` |

### CJK の記号と句読点

全角の空白、句読点、括弧などの区画（U+3000〜U+303F）の文字です。（64 字）

| 文字 | コードポイント | 文字名 | UTF-8 |
|---|---|---|---|
| （空白） | U+3000 | IDEOGRAPHIC SPACE | `E3 80 80` |
| 、 | U+3001 | IDEOGRAPHIC COMMA | `E3 80 81` |
| 。 | U+3002 | IDEOGRAPHIC FULL STOP | `E3 80 82` |
| 〃 | U+3003 | DITTO MARK | `E3 80 83` |
| 〄 | U+3004 | JAPANESE INDUSTRIAL STANDARD SYMBOL | `E3 80 84` |
| 々 | U+3005 | IDEOGRAPHIC ITERATION MARK | `E3 80 85` |
| 〆 | U+3006 | IDEOGRAPHIC CLOSING MARK | `E3 80 86` |
| 〇 | U+3007 | IDEOGRAPHIC NUMBER ZERO | `E3 80 87` |
| 〈 | U+3008 | LEFT ANGLE BRACKET | `E3 80 88` |
| 〉 | U+3009 | RIGHT ANGLE BRACKET | `E3 80 89` |
| 《 | U+300A | LEFT DOUBLE ANGLE BRACKET | `E3 80 8A` |
| 》 | U+300B | RIGHT DOUBLE ANGLE BRACKET | `E3 80 8B` |
| 「 | U+300C | LEFT CORNER BRACKET | `E3 80 8C` |
| 」 | U+300D | RIGHT CORNER BRACKET | `E3 80 8D` |
| 『 | U+300E | LEFT WHITE CORNER BRACKET | `E3 80 8E` |
| 』 | U+300F | RIGHT WHITE CORNER BRACKET | `E3 80 8F` |
| 【 | U+3010 | LEFT BLACK LENTICULAR BRACKET | `E3 80 90` |
| 】 | U+3011 | RIGHT BLACK LENTICULAR BRACKET | `E3 80 91` |
| 〒 | U+3012 | POSTAL MARK | `E3 80 92` |
| 〓 | U+3013 | GETA MARK | `E3 80 93` |
| 〔 | U+3014 | LEFT TORTOISE SHELL BRACKET | `E3 80 94` |
| 〕 | U+3015 | RIGHT TORTOISE SHELL BRACKET | `E3 80 95` |
| 〖 | U+3016 | LEFT WHITE LENTICULAR BRACKET | `E3 80 96` |
| 〗 | U+3017 | RIGHT WHITE LENTICULAR BRACKET | `E3 80 97` |
| 〘 | U+3018 | LEFT WHITE TORTOISE SHELL BRACKET | `E3 80 98` |
| 〙 | U+3019 | RIGHT WHITE TORTOISE SHELL BRACKET | `E3 80 99` |
| 〚 | U+301A | LEFT WHITE SQUARE BRACKET | `E3 80 9A` |
| 〛 | U+301B | RIGHT WHITE SQUARE BRACKET | `E3 80 9B` |
| 〜 | U+301C | WAVE DASH | `E3 80 9C` |
| 〝 | U+301D | REVERSED DOUBLE PRIME QUOTATION MARK | `E3 80 9D` |
| 〞 | U+301E | DOUBLE PRIME QUOTATION MARK | `E3 80 9E` |
| 〟 | U+301F | LOW DOUBLE PRIME QUOTATION MARK | `E3 80 9F` |
| 〠 | U+3020 | POSTAL MARK FACE | `E3 80 A0` |
| 〡 | U+3021 | HANGZHOU NUMERAL ONE | `E3 80 A1` |
| 〢 | U+3022 | HANGZHOU NUMERAL TWO | `E3 80 A2` |
| 〣 | U+3023 | HANGZHOU NUMERAL THREE | `E3 80 A3` |
| 〤 | U+3024 | HANGZHOU NUMERAL FOUR | `E3 80 A4` |
| 〥 | U+3025 | HANGZHOU NUMERAL FIVE | `E3 80 A5` |
| 〦 | U+3026 | HANGZHOU NUMERAL SIX | `E3 80 A6` |
| 〧 | U+3027 | HANGZHOU NUMERAL SEVEN | `E3 80 A7` |
| 〨 | U+3028 | HANGZHOU NUMERAL EIGHT | `E3 80 A8` |
| 〩 | U+3029 | HANGZHOU NUMERAL NINE | `E3 80 A9` |
| 〪 | U+302A | IDEOGRAPHIC LEVEL TONE MARK | `E3 80 AA` |
| 〫 | U+302B | IDEOGRAPHIC RISING TONE MARK | `E3 80 AB` |
| 〬 | U+302C | IDEOGRAPHIC DEPARTING TONE MARK | `E3 80 AC` |
| 〭 | U+302D | IDEOGRAPHIC ENTERING TONE MARK | `E3 80 AD` |
| 〮 | U+302E | HANGUL SINGLE DOT TONE MARK | `E3 80 AE` |
| 〯 | U+302F | HANGUL DOUBLE DOT TONE MARK | `E3 80 AF` |
| 〰 | U+3030 | WAVY DASH | `E3 80 B0` |
| 〱 | U+3031 | VERTICAL KANA REPEAT MARK | `E3 80 B1` |
| 〲 | U+3032 | VERTICAL KANA REPEAT WITH VOICED SOUND MARK | `E3 80 B2` |
| 〳 | U+3033 | VERTICAL KANA REPEAT MARK UPPER HALF | `E3 80 B3` |
| 〴 | U+3034 | VERTICAL KANA REPEAT WITH VOICED SOUND MARK UPPER HALF | `E3 80 B4` |
| 〵 | U+3035 | VERTICAL KANA REPEAT MARK LOWER HALF | `E3 80 B5` |
| 〶 | U+3036 | CIRCLED POSTAL MARK | `E3 80 B6` |
| 〷 | U+3037 | IDEOGRAPHIC TELEGRAPH LINE FEED SEPARATOR SYMBOL | `E3 80 B7` |
| 〸 | U+3038 | HANGZHOU NUMERAL TEN | `E3 80 B8` |
| 〹 | U+3039 | HANGZHOU NUMERAL TWENTY | `E3 80 B9` |
| 〺 | U+303A | HANGZHOU NUMERAL THIRTY | `E3 80 BA` |
| 〻 | U+303B | VERTICAL IDEOGRAPHIC ITERATION MARK | `E3 80 BB` |
| 〼 | U+303C | MASU MARK | `E3 80 BC` |
| 〽 | U+303D | PART ALTERNATION MARK | `E3 80 BD` |
| 〾 | U+303E | IDEOGRAPHIC VARIATION INDICATOR | `E3 80 BE` |
| 〿 | U+303F | IDEOGRAPHIC HALF FILL SPACE | `E3 80 BF` |
