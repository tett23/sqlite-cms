// Vite の開発サーバーで使う DB を、消えていたり古くなったりしていたら書き出し直す（ADR 0062）。
// 開発サーバーは、起動したときに `sqlite-cms build --data-only` で書き出した web/public/db を配る。
// 動かしている途中で DB が消えたり、マイグレーションが増えて DB の形が古くなったりすると、SPA が DB を読めなくなる。
// 書き出しは、毎回すべてのマイグレーションを当てて DB を作るので、書き出し直せば最新の形になる。
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import type { Plugin } from "vite";
import { SqliteFile } from "../src/sqlite";

/** migrations/ の最新の版（`0011_categories.sql` なら `0011`）。なければ null。 */
export function latestMigration(migrationsDir: string): string | null {
  const versions = readdirSync(migrationsDir)
    .map((name) => /^(\d+)_.*\.sql$/.exec(name)?.[1])
    .filter((version): version is string => version !== undefined)
    .sort();
  return versions.at(-1) ?? null;
}

/** 書き出し直す理由。書き出し直さなくてよければ null。 */
export function staleReason(publicDir: string, migrationsDir: string): string | null {
  const manifestPath = path.join(publicDir, "db", "manifest.json");
  if (!existsSync(manifestPath)) return "DB の案内（db/manifest.json）がありません";
  let dbPath: string;
  try {
    const manifest: { db: string } = JSON.parse(readFileSync(manifestPath, "utf8"));
    dbPath = path.join(publicDir, manifest.db);
  } catch {
    return "DB の案内（db/manifest.json）を読めません";
  }
  if (!existsSync(dbPath)) return `DB（${path.relative(publicDir, dbPath)}）がありません`;

  let applied: string | null;
  try {
    const db = new SqliteFile(new Uint8Array(readFileSync(dbPath)));
    applied = db.hasTable("schema_migrations")
      ? db
          .table("schema_migrations")
          .map((row) => String(row.version))
          .sort()
          .at(-1) ?? null
      : null;
  } catch {
    return `DB（${path.relative(publicDir, dbPath)}）を読めません`;
  }
  const latest = latestMigration(migrationsDir);
  if (latest !== null && (applied === null || applied < latest)) {
    return `DB のマイグレーションが古くなっています（${applied ?? "記録なし"}。最新は ${latest}）`;
  }
  return null;
}

/**
 * 開発サーバーの起動のときと、`/db/` への要求のたびに DB を確かめ、消えていたり古くなったりしていたら、rebuild で書き出し直してから応える。
 * rebuild は同期で動かす（書き出し終わるまで要求を待たせる）。
 */
export function devDatabase({
  publicDir,
  migrationsDir,
  rebuild,
  log = console.log,
}: {
  publicDir: string;
  migrationsDir: string;
  rebuild: () => void;
  log?: (message: string) => void;
}): Plugin {
  const ensure = () => {
    const reason = staleReason(publicDir, migrationsDir);
    if (reason === null) return;
    log(`[sqlite-cms] ${reason}。DB を書き出し直します`);
    rebuild();
  };
  return {
    name: "sqlite-cms-dev-database",
    // 開発サーバーでだけ使う。vitest も開発サーバーと同じ形で Vite を動かすが、テストでは DB を書き出さない。
    apply: (_config, env) => env.command === "serve" && env.mode !== "test" && !process.env.VITEST,
    configureServer(server) {
      ensure();
      server.middlewares.use((req, _res, next) => {
        if (req.url?.startsWith("/db/")) ensure();
        next();
      });
    },
  };
}

/** web/ で `npm run data`（sqlite-cms build --data-only）を動かす。SITE_DIR の指定は、環境変数をそのまま引き継ぐ。 */
export function runDataScript(webDir: string) {
  execFileSync("npm", ["run", "data"], { cwd: webDir, stdio: "inherit" });
}
