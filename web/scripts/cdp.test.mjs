import http from "node:http";
import { describe, expect, it, vi } from "vitest";
import { launchChrome, launchOnce, waitForDevTools } from "./cdp.mjs";

describe("launchChrome", () => {
  it("起動に失敗するか DevTools が応えなければ、その Chrome を止めて起動し直す", async () => {
    const chromes = [{ port: 1, kill: vi.fn() }, { port: 2, kill: vi.fn() }];
    const launch = vi
      .fn()
      .mockRejectedValueOnce(new Error("起動に失敗"))
      .mockResolvedValueOnce(chromes[0])
      .mockResolvedValueOnce(chromes[1]);
    const ready = vi.fn(async (chrome) => {
      if (chrome.port === 1) throw new Error("connect ECONNREFUSED");
    });
    const log = vi.fn();
    vi.useFakeTimers();
    const launched = launchChrome({ launch, ready, log });
    await vi.runAllTimersAsync();
    expect(await launched).toBe(chromes[1]);
    vi.useRealTimers();
    expect(launch).toHaveBeenCalledTimes(3);
    expect(chromes[0].kill).toHaveBeenCalledTimes(1);
    expect(chromes[1].kill).not.toHaveBeenCalled();
    expect(log.mock.calls.map(([message]) => message)).toEqual([
      "Chrome を起動できませんでした（1 回目）。起動し直します: 起動に失敗",
      "Chrome を起動できませんでした（2 回目）。起動し直します: connect ECONNREFUSED",
    ]);
  });

  it("決めた回数で起動できなければ、最後の理由を付けて失敗する", async () => {
    const launch = vi.fn().mockRejectedValue(new Error("起動に失敗"));
    vi.useFakeTimers();
    const launched = launchChrome({ attempts: 2, launch, log: () => {} });
    const assertion = expect(launched).rejects.toThrow("Chrome を 2 回起動しても、つなげませんでした: 起動に失敗");
    await vi.runAllTimersAsync();
    await assertion;
    vi.useRealTimers();
    expect(launch).toHaveBeenCalledTimes(2);
  });
});

describe("waitForDevTools", () => {
  it("DevTools の口が応えれば戻り、応えなければ時間を区切って失敗する", async () => {
    const server = http.createServer((req, res) => res.end(req.url === "/json/version" ? "{}" : ""));
    await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
    const { port } = server.address();
    await waitForDevTools(port, 2000);
    await new Promise((resolve) => server.close(resolve));
    await expect(waitForDevTools(port, 500)).rejects.toThrow(`Chrome の DevTools が応えない（ポート ${port}）`);
  });
});

describe("launchOnce", () => {
  it("起動が上限の時間までに終わらなければ、その Chrome を止めて失敗する", async () => {
    const launcher = { launch: () => new Promise(() => {}), kill: vi.fn(), port: 0 };
    await expect(launchOnce({}, 50, () => launcher)).rejects.toThrow("Chrome が 0.05 秒で起動しない");
    expect(launcher.kill).toHaveBeenCalledTimes(1);
  });

  it("起動が終われば、ポートと止める関数を返す。起動を待つ時間を延ばして渡す", async () => {
    let options;
    const launcher = { launch: async () => {}, kill: vi.fn(), port: 9222 };
    const chrome = await launchOnce({ chromeFlags: ["--headless=new"] }, 1000, (o) => ((options = o), launcher));
    expect(chrome.port).toBe(9222);
    chrome.kill();
    expect(launcher.kill).toHaveBeenCalledTimes(1);
    expect(options).toMatchObject({ chromeFlags: ["--headless=new"], connectionPollInterval: 500, maxConnectionRetries: 120 });
  });
});
