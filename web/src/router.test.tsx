import { renderToStaticMarkup } from "preact-render-to-string";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Link, matchPath, navigate, Router, shouldNavigateInApp, usePath, useSearch, type ClickLike } from "./router";

describe("matchPath", () => {
  it("固定のパスは完全一致だけを受け付ける", () => {
    expect(matchPath("/", "/")).toEqual({});
    expect(matchPath("/about", "/about")).toEqual({});
    expect(matchPath("/about", "/about/")).toEqual({});
    expect(matchPath("/about", "/about/me")).toBeNull();
    expect(matchPath("/", "/about")).toBeNull();
  });

  it("パラメータを取り出してデコードする", () => {
    expect(matchPath("/posts/:slug", "/posts/hello")).toEqual({ slug: "hello" });
    expect(matchPath("/posts/:slug", "/posts/%E6%97%A5%E6%9C%AC")).toEqual({ slug: "日本" });
  });

  it("空のパラメータや壊れたエスケープは一致しない", () => {
    expect(matchPath("/posts/:slug", "/posts/")).toBeNull();
    expect(matchPath("/posts/:slug", "/posts/%E6%97")).toBeNull();
    expect(matchPath("/posts/:slug", "/articles/hello")).toBeNull();
  });
});

describe("shouldNavigateInApp", () => {
  const plain: ClickLike = {
    button: 0,
    metaKey: false,
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    defaultPrevented: false,
  };

  it("修飾キーなしの左クリックだけを SPA 内の遷移にする", () => {
    expect(shouldNavigateInApp(plain, undefined)).toBe(true);
    expect(shouldNavigateInApp(plain, "_self")).toBe(true);
  });

  it("新しいタブで開く操作や、ほかの処理が止めたクリックはブラウザに任せる", () => {
    expect(shouldNavigateInApp({ ...plain, metaKey: true }, undefined)).toBe(false);
    expect(shouldNavigateInApp({ ...plain, ctrlKey: true }, undefined)).toBe(false);
    expect(shouldNavigateInApp({ ...plain, shiftKey: true }, undefined)).toBe(false);
    expect(shouldNavigateInApp({ ...plain, altKey: true }, undefined)).toBe(false);
    expect(shouldNavigateInApp({ ...plain, button: 1 }, undefined)).toBe(false);
    expect(shouldNavigateInApp({ ...plain, defaultPrevented: true }, undefined)).toBe(false);
    expect(shouldNavigateInApp(plain, "_blank")).toBe(false);
  });
});

describe("Router と Link", () => {
  function ShowPath() {
    return <span>{usePath()}</span>;
  }

  it("渡したパスを配る", () => {
    const html = renderToStaticMarkup(
      <Router path="/posts/hello">
        <ShowPath />
      </Router>,
    );
    expect(html).toBe("<span>/posts/hello</span>");
  });

  it("渡した問い合わせ（?q=）を配る。渡さなければ空", () => {
    function ShowSearch() {
      return <span>{useSearch()}</span>;
    }
    expect(renderToStaticMarkup(<Router path="/search" search="?q=%E8%A8%98%E4%BA%8B"><ShowSearch /></Router>)).toBe(
      "<span>?q=%E8%A8%98%E4%BA%8B</span>",
    );
    expect(renderToStaticMarkup(<Router path="/search"><ShowSearch /></Router>)).toBe("<span></span>");
  });

  it("Link は href と属性を持つ a 要素になる", () => {
    const html = renderToStaticMarkup(
      <Link to="/about" className="nav">
        自己紹介
      </Link>,
    );
    expect(html).toBe('<a href="/about" class="nav">自己紹介</a>');
  });
});

/** history と、ページを移ったことの知らせを記録する、偽の window。 */
function stubWindow(): string[] {
  const calls: string[] = [];
  vi.stubGlobal("window", {
    history: {
      pushState: (_state: unknown, _title: string, url: string) => calls.push(`push ${url}`),
      replaceState: (_state: unknown, _title: string, url: string) => calls.push(`replace ${url}`),
    },
    dispatchEvent: (event: Event) => calls.push(`event ${event.type}`),
  });
  return calls;
}

describe("navigate", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("履歴を足して移り、ルーターに知らせる", () => {
    const calls = stubWindow();
    navigate("/search?q=a");
    expect(calls).toEqual(["push /search?q=a", "event sqlite-cms:navigate"]);
  });

  it("replace なら履歴を増やさずに置き換え、ルーターに知らせる（ADR 0042）", () => {
    const calls = stubWindow();
    navigate("/search?q=b", { replace: true });
    expect(calls).toEqual(["replace /search?q=b", "event sqlite-cms:navigate"]);
  });
});

describe("Link を押したとき", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  type Click = ClickLike & { preventDefault(): void };
  function click(overrides: Partial<ClickLike> = {}): Click {
    const event = {
      button: 0,
      metaKey: false,
      ctrlKey: false,
      shiftKey: false,
      altKey: false,
      defaultPrevented: false,
      ...overrides,
      preventDefault() {
        event.defaultPrevented = true;
      },
    };
    return event;
  }

  /** Link が描く a 要素の onClick を呼ぶ。 */
  function press(link: ReturnType<typeof Link>, event: Click) {
    (link as unknown as { props: { onClick(event: Click): void } }).props.onClick(event);
  }

  it("普通に押すと、ページを読み直さずに移る", () => {
    const calls = stubWindow();
    const event = click();
    press(Link({ to: "/about", children: "自己紹介" }), event);
    expect(event.defaultPrevented).toBe(true);
    expect(calls).toEqual(["push /about", "event sqlite-cms:navigate"]);
  });

  it("修飾キー付き、中ボタン、新しいタブ（target）では、ブラウザに任せる", () => {
    const calls = stubWindow();
    for (const event of [click({ metaKey: true }), click({ ctrlKey: true }), click({ shiftKey: true }), click({ button: 1 })]) {
      press(Link({ to: "/about" }), event);
      expect(event.defaultPrevented).toBe(false);
    }
    press(Link({ to: "/about", target: "_blank" }), click());
    expect(calls).toEqual([]);
  });

  it("渡した onClick を先に呼び、そこで既定の動きを止めたら移らない", () => {
    const calls = stubWindow();
    const onClick = vi.fn((event: Click) => event.preventDefault());
    press(Link({ to: "/about", onClick }), click());
    expect(onClick).toHaveBeenCalledOnce();
    expect(calls).toEqual([]);
  });
});
