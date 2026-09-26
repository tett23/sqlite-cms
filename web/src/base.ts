// サイトを置くパス（ADR 0030）。GitHub Pages のプロジェクトのページなら "/<リポジトリ名>/"。
// CLI が index.html に <meta name="sqlite-cms-base"> として書き込む。なければ "/"。

function readBasePath(): string {
  if (typeof document === "undefined") return "/";
  const content = document.querySelector('meta[name="sqlite-cms-base"]')?.getAttribute("content");
  return content && content.startsWith("/") && content.endsWith("/") ? content : "/";
}

export const BASE_PATH = readBasePath();

/** サイトの中のパス（"/about" など）を、サイトを置くパスから始まる URL にする。/ で始まらないものと "//" で始まるものはそのまま。 */
export function withBasePath(path: string, basePath: string = BASE_PATH): string {
  return path.startsWith("/") && !path.startsWith("//") ? basePath + path.slice(1) : path;
}

/** URL のパスから、サイトを置くパスを外して、サイトの中のパスにする。サイトの外ならそのまま。 */
export function stripBasePath(pathname: string, basePath: string = BASE_PATH): string {
  if (basePath === "/") return pathname;
  if (pathname.startsWith(basePath)) return "/" + pathname.slice(basePath.length);
  if (pathname + "/" === basePath) return "/";
  return pathname;
}
