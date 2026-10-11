import { describe, expect, it } from "vitest";
import { languageOfFile } from "./highlightLanguages";

describe("languageOfFile（コードの埋め込み、ADR 0065）", () => {
  it("拡張子から、本文のコードブロックで使える言語を決める", () => {
    expect(languageOfFile("main.rs")).toBe("rs");
    expect(languageOfFile("App.TSX")).toBe("tsx");
    expect(languageOfFile("a.b.py")).toBe("py");
    expect(languageOfFile("index.htm")).toBe("html");
    expect(languageOfFile("README.markdown")).toBe("md");
    expect(languageOfFile("changes.diff")).toBe("diff");
  });

  it("当てはまらなければ色を付けない", () => {
    expect(languageOfFile("Makefile")).toBeUndefined();
    expect(languageOfFile(".bashrc")).toBeUndefined();
    expect(languageOfFile("a.unknown")).toBeUndefined();
  });
});
