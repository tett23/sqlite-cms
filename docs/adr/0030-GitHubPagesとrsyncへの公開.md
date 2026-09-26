# 0030 GitHub Pages と rsync への公開

## 状態

採用。
0011 の「公開先は Cloudflare Workers」を、三つから選べる形に改める。

## 決定

`sqlite-cms deploy` の公開先に、GitHub Pages と、rsync で送る任意のサーバーを加える。
公開先は `site.toml` の `[deploy] target` で選ぶ。

```toml
[deploy]
target = "cloudflare"     # 省略したとき。worker を書く
target = "github-pages"   # branch、remote、cname を書ける（どれも任意）
target = "rsync"          # destination を書く
```

- `target` のない、これまでの書き方（`worker` だけ）は、Cloudflare とみなす。
- ほかの公開先の項目を書いたらエラーにする（`target = "rsync"` に `worker` を書いたときなど）。書き間違いに気付けるようにするためである。
- コマンドに渡す値（`branch`、`remote`、`destination`）が `-` で始まるか空白を含むときはエラーにする。オプションと取り違えられないようにするためである。

### サイトを置くパス（base_path）

GitHub Pages のプロジェクトのページは `https://<user>.github.io/<リポジトリ名>/` で配信され、サイトがドメインの直下にない。
これまでの SPA は、アセット、DB、ページ、本文の画像のパスをすべて `/` から書いていたので、そのままでは動かない。
`site.toml` に `base_path`（既定は `/`）を足し、サイトを置くパスを指定できるようにする。

- SPA は相対のパス（Vite の `base: "./"`）でビルドする。JS の中のチャンク、wasm、フォントは、読み込む側の JS や CSS の URL を基準に解決されるので、どこに置いても読める。
- `index.html` だけは、深いパスの URL で 404.html として返されても読めるよう、CLI が組み立てるときにアセットと favicon のパスを `base_path` から始まる形に書き換える。
- CLI は `<meta name="sqlite-cms-base" content="<base_path>">` を `index.html` に入れる。SPA はこれを読み、ルータ（URL とサイトの中のパスの変換）、DB の取得、本文のサイト内のリンク（`/about` など）と画像（`/media/…`、`/link-cards/…`）に `base_path` を付ける。
- 本文には、これまでどおり `/` から始まるパスで書ける。`base_path` を変えても記事を書き直さなくてよい。
- `<base href>` は使わない。脚注のリンク（`#user-content-fn-1`）など、ページの中へのリンクまで `base_path` を基準に解決され、別のページへ移ってしまうためである。
- `serve` も `base_path` の下で配信し、`/` は `base_path` に転送する（302）。
- Cloudflare では `base_path` を使えない（エラーにする）。Worker はドメインの直下で配信し、SPA のフォールバックもそこで働くためである。
- `base_path` は `/` で始まり、英数字と `-`、`_`、`.`、`~`、`/` だけを使える。`//`（別のホストを指す URL と取り違える）、`.` と `..` の区切りは使えない。末尾の `/` は補う。

### SPA のフォールバック

GitHub Pages と一般のサーバーには、Cloudflare の `not_found_handling` のような SPA のフォールバックがない。
`index.html` と同じ中身の `404.html` を、どの公開先でも置く。
GitHub Pages は知らないパスに `404.html` を返すので、記事の URL を直接開いても表示できる（HTTP の状態は 404 になる）。
rsync で送るサーバーには、知らないパスで `index.html` を返す設定を入れるよう、README に nginx と Apache の例を書く。

### GitHub Pages

組み立てた一式を、記事リポジトリの `gh-pages` ブランチに push する。

1. 一式を一時ディレクトリに書き出し、`.nojekyll`（Jekyll で組み立て直させない）と、`cname` を書いたときは `CNAME` を置く。
2. 一時的なインデックス（`GIT_INDEX_FILE`）と、作業ツリーを一時ディレクトリにした `git add --all --force` で、一式だけのツリーを作る（`--force` で、利用者の全体の除外設定に公開用のファイルが漏れないようにする）。
3. `git commit-tree` で、親を持たないコミットを作る。メッセージには記事リポジトリの HEAD のコミットを書く。名前とメールアドレスが設定されていなければ（CI など）、`sqlite-cms` を使う。
4. `git push --force <remote> <コミット>:refs/heads/<branch>` で送る。

記事リポジトリの作業ツリー、インデックス、手元のブランチには触れない。
記事リポジトリの Git の設定をそのまま使うので、手元では利用者の認証情報で、GitHub Actions では `actions/checkout` が設定した認証情報（`permissions: contents: write`）で push できる。
一時的な別のリポジトリを作って push する方法を採らなかったのは、`actions/checkout` の認証情報が記事リポジトリの設定にしかなく、別のリポジトリからは push できないためである。

公開するたびにブランチを作り直す（履歴は残さない）。
独自ドメインを GitHub の画面で設定すると、GitHub がブランチに `CNAME` を置くが、次の公開で消える。そのため `cname` に書いて毎回置く。

GitHub Actions のワークフローの雛形を `example/.github/workflows/deploy-github-pages.yml` に置く。
GitHub の公式の方法（`actions/upload-pages-artifact` と `actions/deploy-pages`）は使わない。`sqlite-cms deploy` を手元でも CI でも同じように使えるようにするためである。
初めて公開した後に、リポジトリの Settings → Pages で公開元を `gh-pages` ブランチにする必要がある。公開の後に、そのことを表示する。

### rsync

組み立てた一式を一時ディレクトリに書き出し、`rsync -rlptz --delete <一時ディレクトリ>/ <destination>/` で送る。
接続には、手元の ssh の設定と鍵をそのまま使う。

`--delete` で、送り先にあって一式にないファイルを消す（古い DB やアセットが残らないようにする）。
送り先を書き間違えたときに関係のないファイルを消さないよう、先に `rsync --list-only` で送り先を確かめる。

- 送り先がない（rsync が作る）か、空なら送る。
- 目印の `.sqlite-cms`（`build --out` の出力先と同じ目印。一式に含まれる）があれば、前に `sqlite-cms` が公開した場所なので送る。
- それ以外はエラーにし、何も送らない。

macOS の openrsync（rsync 2.6.9 互換）でも、`--list-only`、`--delete`、`-rlptz` が使えることを確かめた。

## 実装しないこと

- ほかの公開先（Netlify、Vercel、S3 など）。`build --out` で書き出した一式を、それぞれの方法で置ける。
- GitHub Pages のブランチの履歴を残すこと。
- rsync の追加のオプション（`site.toml` で指定すること）。
- サーバーの設定ファイル（`.htaccess` など）を一式に入れること。サーバーによって書き方が違い、nginx では配信されてしまうので、README に例を書くだけにする。
- Cloudflare での `base_path`。

## テスト設計

- 設定：各公開先の読み方と既定値、`target` のない書き方、知らない `target`、公開先ごとの必須の項目、ほかの公開先の項目、`-` で始まる値と空白を含む値、不正な `cname`。`base_path` の補い方と、不正な形（`/` で始まらない、空、`//`、`..`、`.`、空白、日本語、`?`）。Cloudflare と `base_path` の組み合わせ。
- `index.html` の書き換え（`/` とサブパス）と、`404.html` が `index.html` と同じであること。
- `serve`：`base_path` の下で配信し、`/` を転送し、外のパスは 404 にすること。
- rsync の `--list-only` の出力から名前を取り出すこと（GNU の rsync の大きさの区切り、空白を含む名前）。
- SPA：`base_path` の付け方と外し方（`/` から始まらないもの、`//` で始まるもの、`#` で始まるものはそのまま）。
- 結合テスト
  - rsync：手元のディレクトリに公開でき、二度目は一式にないファイルを消すこと。関係のないファイルのあるディレクトリには送らず、中身を変えないこと。
  - GitHub Pages：手元の空のリモートに push し、`gh-pages` に `index.html`、`404.html`、`.nojekyll`、`CNAME`、DB、画像があり、`index.html` が `base_path` に合わせてあること。記事リポジトリの作業ツリー、HEAD、ブランチが変わらないこと。二度目も公開できること。Git のリポジトリでなければエラーになること。
  - `build` と `serve` が `base_path` を使うこと。

ブラウザでは、`base_path = "/blog/"` の見本を `serve` で配信し、`/` から `/blog/` への転送、リンクと画像とリンクカードのパス、深い URL を直接開いたときの表示、後から読み込む Shiki、KaTeX、mermaid、アプリの中の遷移と戻る操作を確かめた。

## トレードオフ

- 公開先ごとに、`sqlite-cms` が外部のコマンド（`git`、`rsync`）に頼る。見つからなければ、入れるよう案内する。
- GitHub Pages では、記事の URL を直接開くと HTTP の状態が 404 になる。ブラウザでは読めるが、リンクを確かめるツールなどは失敗と見なす。
- `base_path` のために、SPA の中でサイト内のパスを URL にするところは、すべて `withBasePath` を通す必要がある。
