import { describe, expect, it } from "vitest";
import { EXTENSION_LANGUAGES, FILE_NAME_LANGUAGES, languageId, languageOfFile } from "./highlightLanguages";

describe("languageOfFile（コードの埋め込み、ADR 0065、0067）", () => {
  it("ファイル名そのものの表で決める", () => {
    expect(languageOfFile("Dockerfile")).toBe("docker");
    expect(languageOfFile("Makefile")).toBe("make");
    expect(languageOfFile("CMakeLists.txt")).toBe("cmake");
    expect(languageOfFile(".zshrc")).toBe("shellscript");
    expect(languageOfFile("Gemfile")).toBe("ruby");
  });

  it("拡張子が言語の名前か別名に当たれば、その言語にする", () => {
    expect(languageOfFile("main.rs")).toBe("rust");
    expect(languageOfFile("App.TSX")).toBe("tsx");
    expect(languageOfFile("a.b.py")).toBe("python");
    expect(languageOfFile("main.go")).toBe("go");
    expect(languageOfFile("a.rb")).toBe("ruby");
    expect(languageOfFile("changes.diff")).toBe("diff");
  });

  it("名前と別名にない拡張子は、拡張子の表で決める", () => {
    expect(languageOfFile("a.h")).toBe("c");
    expect(languageOfFile("a.cs")).toBe("csharp");
    expect(languageOfFile("main.tf")).toBe("terraform");
    expect(languageOfFile("index.htm")).toBe("html");
    expect(languageOfFile("README.markdown")).toBe("markdown");
  });

  it("当たらなければ色を付けない", () => {
    expect(languageOfFile("LICENSE")).toBeUndefined();
    expect(languageOfFile(".bashrc.bak")).toBeUndefined();
    expect(languageOfFile("a.unknown")).toBeUndefined();
    expect(languageOfFile("diagram.mmd")).toBeUndefined();
  });

  it("表のすべての言語が、Shiki の言語にある", () => {
    for (const language of [...Object.values(FILE_NAME_LANGUAGES), ...Object.values(EXTENSION_LANGUAGES)]) {
      expect(languageId(language), language).toBe(language);
    }
  });
});
