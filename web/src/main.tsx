import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./index.css";
import { highlightLoader } from "./highlightLoader";
import { Router } from "./router";

// DB の取得と並行して読み込み始め、記事の表示までに間に合わせる。
highlightLoader.load().catch(() => {});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Router>
      <App />
    </Router>
  </StrictMode>,
);
