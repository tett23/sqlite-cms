import { describe, expect, it } from "vitest";
import { BASE_PATH, stripBasePath, withBasePath } from "./base";

describe("サイトを置くパス（ADR 0030）", () => {
  it("meta がない環境では / になる", () => {
    expect(BASE_PATH).toBe("/");
  });

  it("サイトの中のパスに、サイトを置くパスを付ける", () => {
    expect(withBasePath("/about", "/blog/")).toBe("/blog/about");
    expect(withBasePath("/", "/blog/")).toBe("/blog/");
    expect(withBasePath("/media/a.png", "/blog/")).toBe("/blog/media/a.png");
    expect(withBasePath("/db/articles-x.sqlite", "/")).toBe("/db/articles-x.sqlite");
  });

  it("/ で始まらないものと // で始まるものはそのまま", () => {
    expect(withBasePath("https://example.com/a.png", "/blog/")).toBe("https://example.com/a.png");
    expect(withBasePath("//cdn.example.com/a.png", "/blog/")).toBe("//cdn.example.com/a.png");
    expect(withBasePath("#footnote", "/blog/")).toBe("#footnote");
    expect(withBasePath("a.png", "/blog/")).toBe("a.png");
  });

  it("URL のパスから、サイトを置くパスを外す", () => {
    expect(stripBasePath("/blog/posts/hello", "/blog/")).toBe("/posts/hello");
    expect(stripBasePath("/blog/", "/blog/")).toBe("/");
    expect(stripBasePath("/blog", "/blog/")).toBe("/");
    expect(stripBasePath("/other", "/blog/")).toBe("/other");
    expect(stripBasePath("/posts/hello", "/")).toBe("/posts/hello");
  });
});
