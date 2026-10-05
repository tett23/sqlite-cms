// Chrome DevTools Protocol（CDP）で、ヘッドレスの Chrome のタブを動かす（E2E のテストと Core Web Vitals の計測で使う）。

export const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/** Chrome に送った一つの命令を待つ時間。Chrome が応えなくなったときに、止まり続けないようにする。 */
export const COMMAND_TIMEOUT = 30000;

/** CDP の一つのタブ。送った命令の結果と、起きた出来事（コンソールのエラー、応答の状態など）を受け取る。 */
export class Tab {
  static async open(port) {
    const signal = AbortSignal.timeout(COMMAND_TIMEOUT);
    const target = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: "PUT", signal })).json();
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

  /** 命令を送り、結果を待つ。Chrome が応えなくなったときに止まり続けないよう、時間を区切る。 */
  send(method, params = {}) {
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
      this.ws.send(JSON.stringify({ id, method, params }));
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

