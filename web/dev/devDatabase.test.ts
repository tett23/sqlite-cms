import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import initSqlJs from "sql.js";
import { describe, expect, it, vi } from "vitest";
import { devDatabase, latestMigration, staleReason } from "./devDatabase";

/** 一時ディレクトリに、public/（DB と案内）と migrations/ を作る。 */
async function fixture({ manifest = true, db = true, applied = ["0001", "0002"] as string[] | null, latest = "0002" } = {}) {
  const root = mkdtempSync(path.join(tmpdir(), "dev-db-"));
  const publicDir = path.join(root, "public");
  const migrationsDir = path.join(root, "migrations");
  mkdirSync(path.join(publicDir, "db"), { recursive: true });
  mkdirSync(migrationsDir);
  writeFileSync(path.join(migrationsDir, "0001_initial.sql"), "");
  writeFileSync(path.join(migrationsDir, `${latest}_latest.sql`), "");
  writeFileSync(path.join(migrationsDir, "README.md"), "");
  if (manifest) writeFileSync(path.join(publicDir, "db", "manifest.json"), JSON.stringify({ db: "/db/articles-0.sqlite" }));
  if (db) {
    const SQL = await initSqlJs();
    const sql = new SQL.Database();
    sql.run("CREATE TABLE posts (slug TEXT PRIMARY KEY)");
    if (applied) {
      sql.run("CREATE TABLE schema_migrations (version TEXT PRIMARY KEY, name TEXT NOT NULL)");
      for (const version of applied) sql.run("INSERT INTO schema_migrations VALUES (?, 'x')", [version]);
    }
    writeFileSync(path.join(publicDir, "db", "articles-0.sqlite"), sql.export());
    sql.close();
  }
  return { publicDir, migrationsDir };
}

describe("staleReason（ADR 0062）", () => {
  it("DB が最新のマイグレーションまで済んでいれば書き出し直さない", async () => {
    const { publicDir, migrationsDir } = await fixture();
    expect(latestMigration(migrationsDir)).toBe("0002");
    expect(staleReason(publicDir, migrationsDir)).toBeNull();
  });

  it("案内や DB が消えていたら書き出し直す", async () => {
    let dirs = await fixture({ manifest: false });
    expect(staleReason(dirs.publicDir, dirs.migrationsDir)).toContain("db/manifest.json）がありません");
    dirs = await fixture({ db: false });
    expect(staleReason(dirs.publicDir, dirs.migrationsDir)).toContain("articles-0.sqlite）がありません");
  });

  it("マイグレーションが増えていたり、記録がなかったりしたら書き出し直す", async () => {
    let dirs = await fixture({ latest: "0011" });
    expect(staleReason(dirs.publicDir, dirs.migrationsDir)).toBe("DB のマイグレーションが古くなっています（0002。最新は 0011）");
    dirs = await fixture({ applied: null });
    expect(staleReason(dirs.publicDir, dirs.migrationsDir)).toContain("記録なし");
  });

  it("壊れた案内と DB は、読めないとして書き出し直す", async () => {
    const { publicDir, migrationsDir } = await fixture();
    writeFileSync(path.join(publicDir, "db", "articles-0.sqlite"), "壊れた DB");
    expect(staleReason(publicDir, migrationsDir)).toContain("を読めません");
    writeFileSync(path.join(publicDir, "db", "manifest.json"), "{");
    expect(staleReason(publicDir, migrationsDir)).toContain("db/manifest.json）を読めません");
  });
});

describe("devDatabase（ADR 0062）", () => {
  /** 開発サーバーの代わり。足された middleware を受け取る。 */
  function fakeServer() {
    const middlewares: ((req: { url?: string }, res: unknown, next: () => void) => void)[] = [];
    return { middlewares, server: { middlewares: { use: (fn: (typeof middlewares)[number]) => middlewares.push(fn) } } };
  }

  it("起動のときと /db/ への要求のときに確かめ、古ければ書き出し直す", async () => {
    const { publicDir, migrationsDir } = await fixture({ manifest: false });
    const rebuild = vi.fn();
    const log = vi.fn();
    const plugin = devDatabase({ publicDir, migrationsDir, rebuild, log });
    const { middlewares, server } = fakeServer();
    (plugin.configureServer as (server: unknown) => void)(server);
    expect(rebuild).toHaveBeenCalledTimes(1);
    expect(log.mock.calls[0][0]).toContain("DB を書き出し直します");

    const next = vi.fn();
    middlewares[0]({ url: "/db/manifest.json" }, {}, next);
    middlewares[0]({ url: "/assets/index.js" }, {}, next);
    expect(rebuild).toHaveBeenCalledTimes(2);
    expect(next).toHaveBeenCalledTimes(2);
  });

  it("DB が新しければ書き出し直さない。テスト（vitest）では使わない", async () => {
    const { publicDir, migrationsDir } = await fixture();
    const rebuild = vi.fn();
    const plugin = devDatabase({ publicDir, migrationsDir, rebuild, log: () => {} });
    const { middlewares, server } = fakeServer();
    (plugin.configureServer as (server: unknown) => void)(server);
    middlewares[0]({ url: "/db/manifest.json" }, {}, () => {});
    expect(rebuild).not.toHaveBeenCalled();

    const apply = plugin.apply as (config: object, env: { command: string; mode: string }) => boolean;
    expect(apply({}, { command: "serve", mode: "test" })).toBe(false);
    expect(apply({}, { command: "build", mode: "production" })).toBe(false);
  });
});
