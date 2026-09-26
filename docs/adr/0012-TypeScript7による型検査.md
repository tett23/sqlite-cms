# 0012 TypeScript 7 による型検査

## 状態

採用。

## 決定

`web/` の TypeScript を 7 系（`typescript@^7.0.2`）に上げる。
7 系の `tsc` は Go で実装されたネイティブバイナリであり、npm のプラットフォームごとの任意依存（`@typescript/typescript-<os>-<cpu>`）として配布される。

TypeScript を使うのは `npm run build` の型検査（`tsc --noEmit`）だけである。
バンドルは vite（esbuild と Rollup）が行い、TypeScript の変換にも型検査にも `typescript` パッケージを使わない。
vitest も同様である。
そのため、この変更で速くなるのは型検査の部分に限られる。

手元（Apple Silicon）での測定では、`tsc --noEmit` が約 0.9 秒から約 0.3 秒に、`npm run build` 全体が約 2.2 秒から約 1.3 秒になった（いずれも `npx` の起動を含む）。

`tsconfig.json` は変更なしで 7 系の型検査を通る。

## 実装しないこと

- vite のバンドラの置き換え。バンドルは TypeScript の版と関係しない。
- 型検査を vite のビルドと並列に走らせること。現在の規模では直列でも十分に短い。

## テスト設計

- `npm run build` の型検査が、既存のコードで成功すること。
- わざと型エラーを入れたファイルで、型検査が失敗すること（手元で一度だけ確認する。常設のテストにはしない）。
- `npm test`（vitest）と、SPA を埋め込んだ `cargo test` が通ること。
- `package-lock.json` に CI の環境（linux-x64）のバイナリが含まれていること。

## トレードオフ

- 7 系の `typescript` パッケージは、5.x までの JavaScript のコンパイラ API（`createProgram` など）を持たない。TypeScript の API に依存するツール（型情報を使う typescript-eslint のルール、ts-morph など）は、7 系に対応するまで導入できない。導入が必要になったら、そのツールだけ 5 系を別に使うかを判断する。
- 型検査のバイナリはプラットフォームごとに配布されるため、対応外のプラットフォームでは開発できない。Linux、macOS、Windows の主要な CPU は含まれている。
