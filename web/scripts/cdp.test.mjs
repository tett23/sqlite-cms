import http from "node:http";
import { describe, expect, it, vi } from "vitest";
import { COMMAND_TIMEOUT, isUnresponsive, launchChrome, launchOnce, POLL_EVALUATE_TIMEOUT, Tab, waitForDevTools } from "./cdp.mjs";

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

/** 偽の CDP の接続。送った命令を覚え、respond で応える（ADR 0071）。 */
function fakeSocket(respond) {
  const listeners = [];
  const socket = {
    sent: [],
    send(data) {
      const message = JSON.parse(data);
      socket.sent.push(message);
      queueMicrotask(() => respond(message, socket));
    },
    addEventListener(type, listener) {
      if (type === "message") listeners.push(listener);
    },
    emit(message) {
      for (const listener of listeners) listener({ data: JSON.stringify(message) });
    },
    reply(message, result = {}) {
      socket.emit({ id: message.id, result });
    },
  };
  return socket;
}

const methods = (socket) => socket.sent.map((m) => m.method);

describe("Tab（ADR 0071）", () => {
  it("goto は、読み込みが終わるまで評価を送らない", async () => {
    vi.useFakeTimers();
    const socket = fakeSocket((message, s) => {
      if (message.method === "Page.navigate") s.reply(message, { frameId: "f" });
      if (message.method === "Runtime.evaluate") s.reply(message, { result: { value: true } });
    });
    const tab = new Tab(0, { id: "t", url: "about:blank" }, socket);
    const going = tab.goto("http://127.0.0.1/a");
    await vi.advanceTimersByTimeAsync(1000);
    expect(methods(socket)).toEqual(["Page.navigate"]);
    socket.emit({ method: "Page.frameNavigated", params: { frame: { id: "f", url: "http://127.0.0.1/a" } } });
    socket.emit({ method: "Page.loadEventFired", params: {} });
    await vi.advanceTimersByTimeAsync(0);
    await going;
    vi.useRealTimers();
    expect(methods(socket)).toEqual(["Page.navigate", "Runtime.evaluate"]);
    expect(tab.url).toBe("http://127.0.0.1/a");
  });

  it("goto は、開けなければその理由で失敗する", async () => {
    const socket = fakeSocket((message, s) => s.reply(message, { frameId: "f", errorText: "net::ERR_CONNECTION_REFUSED" }));
    const tab = new Tab(0, { id: "t" }, socket);
    await expect(tab.goto("http://127.0.0.1:1/")).rejects.toThrow("http://127.0.0.1:1/ を開けない: net::ERR_CONNECTION_REFUSED");
  });

  it("waitFor は、応えなかった評価をやり直し、成り立てば戻る", async () => {
    vi.useFakeTimers();
    let evaluations = 0;
    const socket = fakeSocket((message, s) => {
      if (message.method !== "Runtime.evaluate") return;
      evaluations++;
      if (evaluations >= 2) s.reply(message, { result: { value: true } });
    });
    const tab = new Tab(0, { id: "t" }, socket);
    const waiting = tab.waitFor("ready()", { timeout: 20000 });
    await vi.advanceTimersByTimeAsync(POLL_EVALUATE_TIMEOUT + 200);
    await waiting;
    vi.useRealTimers();
    expect(evaluations).toBe(2);
  });

  it("waitFor は、ずっと応えなければ、応えなかった回数を書いて失敗する", async () => {
    vi.useFakeTimers();
    const socket = fakeSocket(() => {});
    const tab = new Tab(0, { id: "t" }, socket);
    const waiting = expect(tab.waitFor("ready()", { timeout: 10000, message: "準備ができない" })).rejects.toThrow(
      "待っても成り立たない: 準備ができない（Chrome が 2 回応えなかった）",
    );
    await vi.advanceTimersByTimeAsync(11000);
    await waiting;
    vi.useRealTimers();
  });

  it("打ち切ったときの失敗に、式の先頭とタブの URL を書く。打ち切った後に届いた応答は捨てる", async () => {
    vi.useFakeTimers();
    const socket = fakeSocket(() => {});
    const tab = new Tab(0, { id: "t", url: "http://127.0.0.1/articles/syntax" }, socket);
    const evaluating = tab.eval("document.querySelector('main') !== null").catch((error) => error);
    await vi.advanceTimersByTimeAsync(COMMAND_TIMEOUT);
    const error = await evaluating;
    vi.useRealTimers();
    expect(isUnresponsive(error)).toBe(true);
    expect(error.message).toBe("Chrome が 30 秒応えない: Runtime.evaluate（document.querySelector('main') !== null） http://127.0.0.1/articles/syntax");
    expect(() => socket.reply(socket.sent[0], { result: { value: true } })).not.toThrow();
  });

  it("つないだ iframe や Worker は、有効にする命令が失敗しても動き出させる", async () => {
    const socket = fakeSocket((message, s) => {
      if (message.method === "Log.enable") s.emit({ id: message.id, error: { message: "'Log.enable' wasn't found" } });
      else s.reply(message);
    });
    new Tab(0, { id: "t" }, socket);
    socket.emit({ method: "Target.attachedToTarget", params: { sessionId: "s1", targetInfo: { type: "worker", url: "" }, waitingForDebugger: true } });
    await vi.waitFor(() => expect(methods(socket)).toContain("Runtime.runIfWaitingForDebugger"));
    expect(socket.sent.find((m) => m.method === "Runtime.runIfWaitingForDebugger").sessionId).toBe("s1");
  });
});
