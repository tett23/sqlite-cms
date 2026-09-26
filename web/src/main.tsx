import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./index.css";
import { Router } from "./router";

function start() {
  // Shiki、KaTeX、mermaid は、それを使うコードブロック、数式、図が表示されたときに読み込む（ADR 0029）。
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <Router>
        <App />
      </Router>
    </StrictMode>,
  );
}

// index.html の読み込み中の表示を先に描画させてから始める。
// すぐに描画を置き換えると、最初の描画が JS の実行を待つことになる。
requestAnimationFrame(() => setTimeout(start, 0));
