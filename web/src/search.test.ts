import initSqlJs, { type Database } from "sql.js";
import { beforeAll, describe, expect, it } from "vitest";
import { plainText, queryTerms, search, snippet, tokenize, toMatchExpression } from "./search";

let db: Database;

beforeAll(async () => {
  const SQL = await initSqlJs();
  db = new SQL.Database();
  const migrations = import.meta.glob<string>("../../migrations/*.sql", { query: "?raw", import: "default", eager: true });
  for (const file of Object.keys(migrations).sort()) db.exec(migrations[file]);
  const post = (slug: string, title: string, date: string, body: string) =>
    db.run("INSERT INTO posts VALUES (?, ?, ?, ?)", [slug, title, date, body]);
  post("ruby", "ルビを振る", "2026-09-23", "本文に<ruby>振<rt>ふ</rt></ruby>り仮名を付けたいときは、`<ruby>` を使う。");
  post("shiki", "コードの色分け", "2026-09-25", "色分けには **Shiki** を使っていて、VS Code と同じ文法の定義で色を決めている。");
  post("hello", "記事置き場を作った", "2026-09-16", "書いたものを置いておく場所を作った。[仕組み](/articles/x) に書いた。");
  db.run("INSERT INTO articles VALUES (?, ?, ?, ?, ?, ?)", [
    "getting-started",
    "sqlite-cms の使い方",
    "2026-09-26",
    null,
    null,
    "記事リポジトリを作ってから、Cloudflare に公開するまでの手順。\n\n```sh\nsqlite-cms init my-blog\n```",
  ]);
  db.run("INSERT INTO pages VALUES (?, ?, ?)", ["about", "自己紹介", "組版と日本語の文章が好きです。"]);
  db.run("INSERT INTO pages VALUES (?, ?, ?)", ["draft", "下書き", "組版の下書き。URL のないページ。"]);
});

describe("語の分け方", () => {
  it("英数字は語のまま、日本語は 2 文字ずつ重ねて区切る。全角の英数字は半角にし、小文字にする", () => {
    expect(tokenize("記事置き場")).toEqual(["記事", "事置", "置き", "き場"]);
    expect(tokenize("sqlite-cms の使い方")).toEqual(["sqlite", "cms", "の使", "使い", "い方"]);
    expect(tokenize("ＳＨＩＫＩで色")).toEqual(["shiki", "で色"]);
  });

  it("検索する言葉を検索式にする", () => {
    expect(toMatchExpression(queryTerms("使い方"))).toBe('"使い い方"');
    expect(toMatchExpression(queryTerms("sqlite-cms 公開"))).toBe('"sqlite cms*" "公開"');
    expect(toMatchExpression(queryTerms("字"))).toBeNull();
    expect(toMatchExpression(queryTerms("!!!"))).toBeNull();
  });

  it("Markdown の記号を大まかに取り除く", () => {
    expect(plainText("## 見出し\n\n**強調**と [リンク](/a) と ![画像](/b.png)\n\n```sh\ncode\n```")).toBe(
      "見出し 強調と リンク と 画像 code",
    );
  });
});

describe("検索", () => {
  const titles = (query: string) => search(db, query).map((r) => r.title);

  it("日本語、英数字、全角の英数字、複数の語で探せる", () => {
    expect(titles("仮名")).toEqual(["ルビを振る"]);
    expect(titles("shiki")).toEqual(["コードの色分け"]);
    expect(titles("ＳＨＩＫＩ")).toEqual(["コードの色分け"]);
    expect(titles("shi")).toEqual(["コードの色分け"]);
    expect(titles("記事 公開")).toEqual(["sqlite-cms の使い方"]);
  });

  it("題名に語を含むものを先に、あとは新しい順に並べる", () => {
    expect(titles("記事")).toEqual(["記事置き場を作った", "sqlite-cms の使い方"]);
  });

  it("1 文字の日本語でも探せる", () => {
    expect(titles("振")).toEqual(["ルビを振る"]);
  });

  it("隣り合わない語の並びは一致しない", () => {
    expect(titles("置き方")).toEqual([]);
  });

  it("コードブロックの中も探せる", () => {
    expect(titles("my-blog")).toEqual(["sqlite-cms の使い方"]);
  });

  it("自己紹介を探せ、URL のないページは出さない", () => {
    const results = search(db, "組版");
    expect(results.map((r) => [r.title, r.path])).toEqual([["自己紹介", "/about"]]);
  });

  it("結果にサイトの中のパスと抜き出しを付ける", () => {
    const [result] = search(db, "文法");
    expect(result.path).toBe("/posts/shiki");
    expect(result.snippet.filter((part) => part.match).map((part) => part.text)).toEqual(["文法"]);
  });

  it("空の言葉では何も返さない", () => {
    expect(search(db, "  ")).toEqual([]);
  });
});

describe("抜き出し", () => {
  it("最初に語が現れるあたりを抜き出し、語に印を付ける", () => {
    const text = "あ".repeat(50) + "組版" + "い".repeat(100) + "組版";
    const parts = snippet(text, ["組版"]);
    expect(parts[0]).toEqual({ text: "…", match: false });
    expect(parts.find((part) => part.match)?.text).toBe("組版");
    expect(parts[parts.length - 1]).toEqual({ text: "…", match: false });
  });

  it("語がなければ先頭から抜き出す", () => {
    expect(snippet("短い本文", ["なし"])).toEqual([{ text: "短い本文", match: false }]);
  });
});
