// example などの記事リポジトリを sqlite-cms serve で配信する（Lighthouse の計測と E2E のテストで使う）。
import { spawn } from "node:child_process";
import path from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";

const webDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

/**
 * siteDir を配信し、{ origin, stop } を返す。
 * SQLITE_CMS_BIN を指定すると、そのバイナリ（リリースのビルドなど）で配信する。なければ cargo run で起動する。
 */
export function startServer(siteDir) {
  // 自動反映の接続（Server-Sent Events）が開いたままだと、計測やテストの終わりを待たせるので、使わない（ADR 0034）。
  const serve = ["serve", siteDir, "--port", "0", "--no-reload"];
  const [command, args] = process.env.SQLITE_CMS_BIN
    ? [process.env.SQLITE_CMS_BIN, serve]
    : ["cargo", ["run", "--quiet", "--manifest-path", path.join(webDir, "../cli/Cargo.toml"), "--", ...serve]];
  const child = spawn(command, args, { stdio: ["ignore", "pipe", "inherit"] });
  return new Promise((resolve, reject) => {
    child.on("exit", (code) => reject(new Error(`sqlite-cms serve が終了しました（${code}）`)));
    createInterface({ input: child.stdout }).once("line", (line) => {
      const origin = line.match(/^(http:\/\/[^/\s]+)/)?.[1];
      if (!origin) return reject(new Error(`URL が出力されていません: ${line}`));
      resolve({ origin, stop: () => child.kill() });
    });
  });
}
