// Chrome DevTools Protocol（CDP）で、ヘッドレスの Chrome のタブを動かす（E2E のテストと Core Web Vitals の計測で使う）。
// Chrome の起動（launchChrome）は、Lighthouse の計測でも使う。
import * as chromeLauncher from "chrome-launcher";

export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** Chrome を起動する回数の上限。 */
export const CHROME_LAUNCH_ATTEMPTS = 3;

/** 起動した Chrome の DevTools の口（/json/version）が応えるまで待つ時間。 */
const DEVTOOLS_READY_TIMEOUT = 10000;

/**
 * Chrome の DevTools の口が開くまで、chrome-launcher が待つ時間（0.5 秒ごとに確かめる）。既定の 25 秒では、
 * CI の機械で Chrome を三つ同時に起動したとき（Lighthouse の計測）に、間に合わないことがあった。
 */
const CHROME_CONNECTION_POLL_INTERVAL = 500;
const CHROME_CONNECTION_RETRIES = 120;

/** 一回の起動の上限。起動が戻らないまま止まることがあったので（v0.4.6 の CI で、6 分の制限まで止まった）、過ぎたら止めて起動し直す。 */
export const CHROME_LAUNCH_TIMEOUT = 90000;

/**
 * Chrome を一回起動する。timeout を過ぎても起動が終わらなければ、その Chrome を止めて失敗にする。
 * chrome-launcher の launch() は、終わるまで Chrome を止める手段を返さないので、Launcher を使う。createLauncher はテストで差し替える。
 */
export async function launchOnce(options, timeout = CHROME_LAUNCH_TIMEOUT, createLauncher = (o) => new chromeLauncher.Launcher(o)) {
  const launcher = createLauncher({
    connectionPollInterval: CHROME_CONNECTION_POLL_INTERVAL,
    maxConnectionRetries: CHROME_CONNECTION_RETRIES,
    ...options,
  });
  let timer;
  try {
    await Promise.race([
      launcher.launch(),
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(`Chrome が ${timeout / 1000} 秒で起動しない`)), timeout);
      }),
    ]);
  } catch (error) {
    try {
      launcher.kill();
    } catch {
      // 止められなくても、失敗として返す。
    }
    throw error;
  } finally {
    clearTimeout(timer);
  }
  return { port: launcher.port, kill: () => launcher.kill() };
}

/** Chrome の DevTools の口に応えるまで待つ。応えなければ例外にする。 */
export async function waitForDevTools(port, timeout = DEVTOOLS_READY_TIMEOUT) {
  const start = Date.now();
  let lastError;
  while (Date.now() - start < timeout) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/version`, { signal: AbortSignal.timeout(2000) });
      if (response.ok) return;
      lastError = new Error(`DevTools が ${response.status} を返した`);
    } catch (error) {
      lastError = error;
    }
    await sleep(200);
  }
  throw new Error(`Chrome の DevTools が応えない（ポート ${port}）: ${lastError?.message ?? "時間切れ"}`, { cause: lastError });
}

/**
 * ヘッドレスの Chrome を起動し、DevTools の口に応えることを確かめて返す。
 * 起動に失敗するか応えなければ、その Chrome を止めて、間を空けて起動し直す（attempts 回まで）。
 * CI の機械では、起動した直後の Chrome につなげないことがまれにある（v0.4.5 の CI で、計測の前に ECONNREFUSED で止まった）。
 * launch と ready はテストで差し替える。
 */
export async function launchChrome({
  attempts = CHROME_LAUNCH_ATTEMPTS,
  launch = launchOnce,
  ready = (chrome) => waitForDevTools(chrome.port),
  log = console.error,
} = {}) {
  let lastError;
  for (let attempt = 1; attempt <= attempts; attempt++) {
    let chrome;
    try {
      chrome = await launch({ chromeFlags: ["--headless=new", ...(process.env.CI ? ["--no-sandbox"] : [])] });
      await ready(chrome);
      return chrome;
    } catch (error) {
      lastError = error;
      try {
        chrome?.kill();
      } catch {
        // 止められなくても、起動し直す。
      }
      if (attempt < attempts) {
        log(`Chrome を起動できませんでした（${attempt} 回目）。起動し直します: ${error?.message ?? error}`);
        await sleep(1000 * attempt);
      }
    }
  }
  throw new Error(`Chrome を ${attempts} 回起動しても、つなげませんでした: ${lastError?.message ?? lastError}`, { cause: lastError });
}

/** fn を、接続を拒まれたとき（ECONNREFUSED）だけ、間を空けてやり直す。 */
async function retryOnRefused(fn, attempts = 3) {
  for (let attempt = 1; ; attempt++) {
    try {
      return await fn();
    } catch (error) {
      const refused = error?.cause?.code === "ECONNREFUSED" || error?.code === "ECONNREFUSED";
      if (!refused || attempt >= attempts) throw error;
      await sleep(500 * attempt);
    }
  }
}

/** Chrome に送った一つの命令を待つ時間。Chrome が応えなくなったときに、止まり続けないようにする。 */
export const COMMAND_TIMEOUT = 30000;

/** CDP の一つのタブ。送った命令の結果と、起きた出来事（コンソールのエラー、応答の状態など）を受け取る。 */
export class Tab {
  static async open(port) {
    const signal = AbortSignal.timeout(COMMAND_TIMEOUT);
    const target = await retryOnRefused(async () =>
      (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: "PUT", signal })).json(),
    );
    const tab = new Tab(port, target);
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error("Chrome のタブにつながらない")), COMMAND_TIMEOUT);
      tab.ws.addEventListener("open", () => (clearTimeout(timer), resolve()));
      tab.ws.addEventListener("error", (error) => (clearTimeout(timer), reject(error)));
    });
    await tab.send("Page.enable");
    await tab.send("Runtime.enable");
    await tab.send("Network.enable");
    await tab.send("Log.enable");
    // 別のプロセスで動く iframe（図を描く作業用のページ、ADR 0055）にもつなぎ、その中のエラーと読み込みも集める。
    await tab.send("Target.setAutoAttach", { autoAttach: true, waitForDebuggerOnStart: true, flatten: true });
    // E2E_CPU_THROTTLE（倍率）で CPU を遅くし、遅い CI のマシンでも待ち時間が足りるかを手元で確かめられる。
    const throttle = Number(process.env.E2E_CPU_THROTTLE ?? 1);
    if (throttle > 1) await tab.send("Emulation.setCPUThrottlingRate", { rate: throttle });
    // レイアウトのずれ（Cumulative Layout Shift の元）を集める。入力の直後のずれは数えない（CLS と同じ）。
    await tab.send("Page.addScriptToEvaluateOnNewDocument", {
      source: `window.__layoutShift = 0; new PerformanceObserver((list) => { for (const e of list.getEntries()) if (!e.hadRecentInput) window.__layoutShift += e.value; }).observe({ type: "layout-shift", buffered: true });`,
    });
    return tab;
  }

  constructor(port, target) {
    this.port = port;
    this.id = target.id;
    this.ws = new WebSocket(target.webSocketDebuggerUrl);
    this.nextId = 0;
    this.pending = new Map();
    /** コンソールのエラー、捕まえていない例外、400 以上の応答。 */
    this.problems = [];
    /** 読み込んだ URL。 */
    this.requests = [];
    this.ws.addEventListener("message", (event) => {
      const message = JSON.parse(event.data);
      if (message.id !== undefined) {
        const { resolve, reject } = this.pending.get(message.id);
        this.pending.delete(message.id);
        if (message.error) reject(new Error(message.error.message));
        else resolve(message.result);
        return;
      }
      const { method, params } = message;
      if (method === "Target.attachedToTarget") {
        const { sessionId } = params;
        void Promise.all(["Runtime.enable", "Network.enable", "Log.enable"].map((m) => this.send(m, {}, sessionId)))
          .then(() => this.send("Runtime.runIfWaitingForDebugger", {}, sessionId))
          .catch(() => {});
        return;
      }
      if (method === "Runtime.exceptionThrown") this.problems.push(`例外: ${params.exceptionDetails.exception?.description ?? params.exceptionDetails.text}`);
      if (method === "Runtime.consoleAPICalled" && params.type === "error") {
        this.problems.push(`console.error: ${params.args.map((a) => a.value ?? a.description).join(" ")}`);
      }
      if (method === "Log.entryAdded" && params.entry.level === "error") this.problems.push(`${params.entry.text} ${params.entry.url ?? ""}`);
      if (method === "Network.requestWillBeSent") this.requests.push(params.request.url);
      if (method === "Network.responseReceived" && params.response.status >= 400) {
        this.problems.push(`${params.response.status} ${params.response.url}`);
      }
    });
  }

  /** 命令を送り、結果を待つ。Chrome が応えなくなったときに止まり続けないよう、時間を区切る。sessionId は、つないだ iframe に送るとき。 */
  send(method, params = {}, sessionId = undefined) {
    const id = ++this.nextId;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        reject(new Error(`Chrome が ${COMMAND_TIMEOUT / 1000} 秒応えない: ${method}`));
      }, COMMAND_TIMEOUT);
      this.pending.set(id, {
        resolve: (value) => (clearTimeout(timer), resolve(value)),
        reject: (error) => (clearTimeout(timer), reject(error)),
      });
      this.ws.send(JSON.stringify({ id, method, params, sessionId }));
    });
  }

  /** 式を評価して値を返す。例外なら失敗にする。 */
  async eval(expression) {
    const result = await this.send("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
    if (result.exceptionDetails) throw new Error(`${expression}: ${result.exceptionDetails.exception?.description ?? result.exceptionDetails.text}`);
    return result.result.value;
  }

  /** 条件が成り立つまで待つ。成り立たなければ失敗にする。 */
  async waitFor(expression, { timeout = 10000, message = expression } = {}) {
    const start = Date.now();
    while (Date.now() - start < timeout) {
      if (await this.eval(expression)) return;
      await sleep(100);
    }
    throw new Error(`待っても成り立たない: ${message}`);
  }

  async size(width, height) {
    await this.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: width < 600 });
  }

  /** 開き、本文が描かれる（読み込み中の表示が消える）まで待つ。 */
  async goto(url) {
    this.problems = [];
    this.requests = [];
    await this.send("Page.navigate", { url });
    await this.waitFor("document.readyState === 'complete' && !document.querySelector('.loading') && document.querySelector('main') !== null", {
      message: `${url} の本文が描かれない`,
    });
  }

  async type(text) {
    await this.send("Input.insertText", { text });
    await sleep(200);
  }

  async key(key) {
    const codes = { ArrowDown: 40, ArrowUp: 38, Enter: 13, Escape: 27 };
    await this.send("Input.dispatchKeyEvent", { type: "keyDown", key, code: key, windowsVirtualKeyCode: codes[key], text: key === "Enter" ? "\r" : undefined });
    await this.send("Input.dispatchKeyEvent", { type: "keyUp", key, code: key, windowsVirtualKeyCode: codes[key] });
    await sleep(200);
  }

  async close() {
    this.ws.close();
    await fetch(`http://127.0.0.1:${this.port}/json/close/${this.id}`, { signal: AbortSignal.timeout(5000) });
  }
}

