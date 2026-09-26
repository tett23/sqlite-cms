import { useEffect, useState } from "react";

/** 画面のこの距離まで近づいたら、見えたとみなす。スクロールして見える前に読み込みを始めるため。 */
const MARGIN = "200px 0px";

/**
 * 要素が画面の近くに来たら true を返す（一度 true になったら戻さない）。
 * 画面の外のコードブロック、数式、図のために、大きなライブラリを読み込まないようにする（ADR 0038）。
 * IntersectionObserver がない環境では、すぐに true にする。
 */
export function useNearViewport<T extends Element>(): [(element: T | null) => void, boolean] {
  const [element, setElement] = useState<T | null>(null);
  // IntersectionObserver がない環境（サーバーでの描画、テスト）では、最初から近いとみなす。
  const [near, setNear] = useState(() => typeof IntersectionObserver === "undefined");
  useEffect(() => {
    if (near || !element) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setNear(true);
          observer.disconnect();
        }
      },
      { rootMargin: MARGIN },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [element, near]);
  return [setElement, near];
}
