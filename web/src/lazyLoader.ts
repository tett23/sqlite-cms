import { useEffect, useState } from "react";

/** 大きなライブラリを、本体とは別のチャンクにして必要になってから読み込む。 */
export interface LazyLoader<T> {
  /** 読み込み済みなら返す。まだなら null。 */
  loaded(): T | null;
  /** 読み込みを始める（読み込み中なら同じ Promise を返す）。失敗したら次の呼び出しで読み込み直す。 */
  load(): Promise<T>;
}

export function createLoader<T>(importer: () => Promise<T>): LazyLoader<T> {
  let loaded: T | null = null;
  let loading: Promise<T> | null = null;
  return {
    loaded: () => loaded,
    load() {
      loading ??= importer().then(
        (module) => {
          loaded = module;
          return module;
        },
        (error: unknown) => {
          loading = null;
          throw error;
        },
      );
      return loading;
    },
  };
}

/**
 * 描画に使うときに読み込みを始め、読み込み終わったら描き直す。読み込むまでは null。
 * 読み込めなかったときは onError を呼び、null のままにする（読み込み前の表示が残る）。
 */
export function useLazy<T>(loader: LazyLoader<T>, onError?: (error: unknown) => void): T | null {
  const [module, setModule] = useState(loader.loaded);
  useEffect(() => {
    if (module) return;
    let mounted = true;
    loader.load().then(
      (loaded) => mounted && setModule(() => loaded),
      (error: unknown) => onError?.(error),
    );
    return () => {
      mounted = false;
    };
  }, [loader, module, onError]);
  return module;
}
