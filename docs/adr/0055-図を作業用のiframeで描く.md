# 0055 図を作業用の iframe で描く

## 状態

提案。
0053 で残した「図のページの、mermaid の使っていない JS の警告」を直す。
0025、0038 の「mermaid を、図のあるページで後から読み込んで描く」を、作業用の iframe の中で描く形に改める。

## 背景

Lighthouse（と PageSpeed Insights）の「使用していない JavaScript の削減」が、図のページ（`/articles/heavy-diagrams`、`/articles/complex-mixed`）で、mermaid の二つのチャンクに出ていた（公開先の Brotli でも 23〜25 KiB。基準は 20 KiB）。
二つのチャンクは、mermaid が配布の時点で分けている、どの種類の図でも使う共通の部分で、分けることも削ることもできない。
mermaid は図の文字の大きさを DOM で測るので、Web Worker（0046、0053）にも移せない。

Lighthouse の実装（`js-usage.js`）を読み、使っていない JS は、ページの本体のプロセス（`defaultSession`）のカバレッジだけから数えることを確かめた。
小さな見本で、同じオリジンの iframe の中の JS は数えられ（202 KiB）、オリジンを持たない sandbox の iframe（別のプロセスで動く）の中の JS は数えられないことを確かめた。

## 決定

図を、作業用のページ（`web/mermaid-frame.html`。Vite の二つ目の入口）を読み込んだ iframe の中の mermaid で描き、SVG を受け取って本体のページに置く。

- iframe には `sandbox="allow-scripts"` だけを付ける。`allow-same-origin` を付けないので、オリジンを持たず、本体と別のプロセスで動く。mermaid の読み込みと図の配置が、本体の入力への応答を止めない。
- 図が画面の近くに来たときに、iframe を一つだけ作る（`web/src/render/loaders.ts` の `mermaidLoader`）。画面の外に置き、大きさを持たせる（`display: none` では文字の大きさを測れない）。読み上げからは隠す。
- やり取りは `postMessage` で行う。本体は図の文字列を送り、作業用のページは SVG か誤りを返す。それぞれ、相手の窓（`event.source`）からのものだけを受ける。SVG は、これまでと同じく mermaid の `securityLevel: "strict"` で作ったものを置く。
- 作業用のページは、mermaid を読み込む前に「頼みを受けられる」と知らせ、mermaid は最初の頼みで読み込む。文字の大きさを測る基準（`font-family`、`line-height`）は、本体のページ（Tailwind の preflight）とそろえる。図の大きさ（`viewBox`）は、本体で描いたときと 10 個すべて一致した。

### CORS

オリジンを持たない iframe から `/assets/` の JS（モジュール）を読むので、オリジンの違う読み込みになる。

- Cloudflare：CLI が書き出す `_headers` に、`/assets/*` の `Access-Control-Allow-Origin: *` を足す。
- `sqlite-cms serve`：`/assets/` の下に同じヘッダを付ける。
- GitHub Pages：すべてのファイルに同じヘッダが付いている。
- rsync：送り先のサーバーで付ける（README に nginx と Apache の例を書く）。

### iframe が使えないとき

作業用のページが知らせてこなければ、iframe を外し、これまでどおり本体のページで描く。

- 知らせは iframe の `load` より前に届く。iframe を止められた（広告を止める拡張など）ときや、CORS のヘッダがなくて JS を読めなかったときも `load` は来るので、`load` から 1 秒待って本体で描く。念のため、10 秒で打ち切る。
- アプリの中のブラウザ（iframe を止める）で、最初の図が 1.3 秒で描かれることを確かめた（1 秒で切り替える前は 10 秒以上かかった）。CORS のヘッダを付けないサーバーでも、本体で描かれることを確かめた。

### 依存関係ツリー

iframe にしただけでは、図のページで「ネットワークの依存関係ツリー」（0053）が失敗した。
iframe の中でスクリプトが読み込んだ mermaid のチャンクは、Lighthouse の記録で親（iframe）をたどれず、本体の HTML から直接読んだ、優先度の高い読み込みとみなされたためである（先読みの印も付かない）。
Lighthouse は、優先度の高くない読み込みを重要な連鎖に数えない。作業用のページの中では、Vite が動的な import の前に足す modulepreload のリンクに `fetchpriority="low"` を付ける（`HTMLHeadElement.prototype.appendChild` を包む）。本体の描画には要らない読み込みなので、本体と帯域を取り合わないようにするのにも合う。

### 結果

手元の Lighthouse で、14 ページすべてで、使っていない JS と依存関係ツリーの監査が合格になった。
図のページの Total Blocking Time は、`/articles/heavy-diagrams` で 190 ms から 10 ms に、`/articles/complex-mixed` で 130 ms から 10 ms になった。
アクセシビリティとベストプラクティスは 100 点のまま、Core Web Vitals はすべて「良好」のままだった。

## 実装しないこと

- Vite の manifest から、図の種類ごとのチャンクを引いて先読みすること。低い優先度の先読みで足りた。
- 図をビルドのときに SVG にすること。CLI がヘッドレスのブラウザで mermaid を動かす必要がある。

## テスト設計

- E2E：図のページで、sandbox の iframe が残っている（iframe で描いた）こと。elk を、図で指定したときだけ読み込むこと（iframe の中の読み込みも集める）。iframe の中のコンソールのエラーと 400 以上の応答も、エラーとして数える。そのため、E2E と Core Web Vitals の計測の CDP は、別のプロセスの iframe にもつなぐ（`Target.setAutoAttach`）。
- CLI：`_headers` と、Cloudflare に送る設定に、`/assets/*` の `Access-Control-Allow-Origin` があること。`serve` が `/assets/` の下（`base_path` の下を含む）だけにヘッダを付けること。
- 手で確かめたこと：CORS のヘッダのないサーバーと、iframe を止めるブラウザで、本体で描くこと。iframe で描いた図と本体で描いた図の大きさが一致すること。

## トレードオフ

- 初めて図を描くときに、作業用のページ（HTML と小さな JS）の読み込みと、やり取りの分だけ遅れる。
- `/assets/` をどのオリジンからも読めるようにする。中身は公開している JS で、読み込みに資格情報を使わない。
- rsync の送り先でヘッダを付けていないと、本体で描く（これまでと同じ）。そのときは iframe の `load` から 1 秒、図が遅れる。
- 作業用のページの中で、`appendChild` を包む。Vite の先読みの仕組みが変わると、依存関係ツリーの監査が再び失敗することがある（描画には影響しない）。
