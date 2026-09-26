import { renderToStaticMarkup } from "preact-render-to-string";
import { describe, expect, it } from "vitest";
import { Link, matchPath, Router, shouldNavigateInApp, usePath, type ClickLike } from "./router";

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

  it("Link は href と属性を持つ a 要素になる", () => {
    const html = renderToStaticMarkup(
      <Link to="/about" className="nav">
        自己紹介
      </Link>,
    );
    expect(html).toBe('<a href="/about" class="nav">自己紹介</a>');
  });
});
