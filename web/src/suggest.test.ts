import initSqlJs from "sql.js";
import { beforeAll, describe, expect, it } from "vitest";
import { moveActive, suggest, SUGGESTION_LIMIT } from "./suggest";
import { SqliteFile } from "./sqlite";

let db: SqliteFile;

beforeAll(async () => {
  const SQL = await initSqlJs();
  const sql = new SQL.Database();
  const migrations = import.meta.glob<string>("../../migrations/*.sql", { query: "?raw", import: "default", eager: true });
  for (const file of Object.keys(migrations).sort()) sql.exec(migrations[file]);
  for (let day = 1; day <= 7; day++) {
    sql.run("INSERT INTO posts (slug, title, published_at, body_md) VALUES (?, ?, ?, ?)", [`p${day}`, `メモ ${day}`, `2026-09-0${day}`, "組版についてのメモ。"]);
  }
  sql.run("INSERT INTO posts (slug, title, published_at, body_md) VALUES (?, ?, ?, ?)", ["title", "組版の題", "2026-08-01", "本文。"]);
  // sql.js で作った DB を書き出し、ページの表示と同じく自前の読み手で開く（ADR 0047）。
  db = new SqliteFile(sql.export());
  sql.close();
});

describe("候補", () => {
  it("先頭の SUGGESTION_LIMIT 件だけを、題名に言葉を含むものを先に返す", () => {
    const results = suggest(db, "組版");
    expect(results).toHaveLength(SUGGESTION_LIMIT);
    expect(results.map((r) => r.path)).toEqual(["/posts/title", "/posts/p7", "/posts/p6", "/posts/p5", "/posts/p4"]);
  });

  it("一致するものがなければ空", () => {
    expect(suggest(db, "存在しない言葉")).toEqual([]);
  });

  it("言葉が空か空白だけなら、探さずに空を返す", () => {
    expect(suggest(db, "")).toEqual([]);
    expect(suggest(db, "   ")).toEqual([]);
  });
});

describe("矢印のキーで選ぶ候補", () => {
  it("下の矢印で先頭から順に進み、最後の次は入力欄（-1）に戻る", () => {
    expect([-1, 0, 1, 2].map((active) => moveActive(active, 1, 3))).toEqual([0, 1, 2, -1]);
  });

  it("上の矢印で逆に進み、入力欄の前は最後の候補になる", () => {
    expect([-1, 0, 1, 2].map((active) => moveActive(active, -1, 3))).toEqual([2, -1, 0, 1]);
  });

  it("候補がなければ、どれも選ばない", () => {
    expect(moveActive(-1, 1, 0)).toBe(-1);
    expect(moveActive(-1, -1, 0)).toBe(-1);
  });
});
