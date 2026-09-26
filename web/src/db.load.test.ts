import { afterEach, describe, expect, it, vi } from "vitest";

// DB の取得の順番だけを確かめるので、DB の読み手（ADR 0047）は、受け取ったバイト列を持つだけのものにする。
vi.mock("./sqlite", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./sqlite")>()),
  SqliteFile: class {
    constructor(public bytes: Uint8Array) {}
  },
}));

const { loadDb } = await import("./db");

type FakeResponse = { ok: boolean; status: number; arrayBuffer(): Promise<ArrayBuffer>; json(): Promise<unknown> };

function response(body: string, status = 200): FakeResponse {
  return {
    ok: status >= 200 && status < 300,
    status,
    arrayBuffer: async () => new TextEncoder().encode(body).buffer as ArrayBuffer,
    json: async () => JSON.parse(body),
  };
}

/** index.html の DB のパス（なければ null）と、URL ごとの応答（Error なら通信の失敗）を用意し、取得した URL を返す。 */
function stub(embedded: string | null, responses: Record<string, FakeResponse | Error>): string[] {
  const requested: string[] = [];
  vi.stubGlobal("document", {
    querySelector: (selector: string) =>
      selector === 'meta[name="sqlite-cms-db"]' && embedded ? { getAttribute: () => embedded } : null,
  });
  vi.stubGlobal("fetch", async (url: string) => {
    requested.push(url);
    const result = responses[url] ?? response("", 404);
    if (result instanceof Error) throw result;
    return result;
  });
  return requested;
}

const bytesOf = (db: unknown) => new TextDecoder().decode((db as { bytes: Uint8Array }).bytes);
const manifest = response('{"db":"/db/articles-new.sqlite"}');

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("DB の読み込み（ADR 0038）", () => {
  it("index.html に DB のパスがあれば、マニフェストを読まずに取りに行く", async () => {
    const requested = stub("/db/articles-a.sqlite", { "/db/articles-a.sqlite": response("A") });
    expect(bytesOf(await loadDb())).toBe("A");
    expect(requested).toEqual(["/db/articles-a.sqlite"]);
  });

  it("index.html の DB が取れなければ（消えた古い DB など）、マニフェストから読み直す", async () => {
    const requested = stub("/db/articles-old.sqlite", {
      "/db/manifest.json": manifest,
      "/db/articles-new.sqlite": response("NEW"),
    });
    expect(bytesOf(await loadDb())).toBe("NEW");
    expect(requested).toEqual(["/db/articles-old.sqlite", "/db/manifest.json", "/db/articles-new.sqlite"]);
  });

  it("index.html の DB の取得が通信で失敗しても、マニフェストから読み直す", async () => {
    stub("/db/articles-a.sqlite", {
      "/db/articles-a.sqlite": new TypeError("Failed to fetch"),
      "/db/manifest.json": manifest,
      "/db/articles-new.sqlite": response("NEW"),
    });
    expect(bytesOf(await loadDb())).toBe("NEW");
  });

  it("index.html に DB のパスがなければ（開発サーバー）、マニフェストから読む", async () => {
    const requested = stub(null, { "/db/manifest.json": manifest, "/db/articles-new.sqlite": response("NEW") });
    expect(bytesOf(await loadDb())).toBe("NEW");
    expect(requested).toEqual(["/db/manifest.json", "/db/articles-new.sqlite"]);
  });

  it("マニフェストか DB が取れなければ、状態を添えて失敗する", async () => {
    stub(null, { "/db/manifest.json": response("", 500) });
    await expect(loadDb()).rejects.toThrow("manifest の取得に失敗しました (500)");
    stub(null, { "/db/manifest.json": manifest });
    await expect(loadDb()).rejects.toThrow("DB の取得に失敗しました (404)");
  });
});
