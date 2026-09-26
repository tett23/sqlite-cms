import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./index.css";
import { highlightLoader } from "./highlightLoader";
import { Router } from "./router";

function start() {
  // DB の取得と並行して読み込み始め、記事の表示までに間に合わせる。
  highlightLoader.load().catch(() => {});

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
