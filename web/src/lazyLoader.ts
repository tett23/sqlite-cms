import { useEffect, useSyncExternalStore } from "react";

/**
 * 大きなライブラリを、本体とは別のチャンクにして、必要になったときに初めて読み込む（ADR 0029）。
 * 読み込みを始めるのは、それを使う部品（コードブロック、数式、図）が表示されたときだけにする。
 */
export interface LazyLoader<T> {
  /** 読み込み済みなら返す。まだなら null。 */
  loaded(): T | null;
  /** 読み込みを始める（読み込み中なら同じ Promise を返す）。失敗したら次の呼び出しで読み込み直す。 */
  load(): Promise<T>;
  /** 読み込み終わったときに呼ぶ関数を登録する。登録を解く関数を返す。 */
  subscribe(listener: () => void): () => void;
}

export function createLoader<T>(importer: () => Promise<T>): LazyLoader<T> {
  let loaded: T | null = null;
  let loading: Promise<T> | null = null;
  const listeners = new Set<() => void>();
  return {
    loaded: () => loaded,
    load() {
      loading ??= importer().then(
        (module) => {
          loaded = module;
          for (const listener of listeners) listener();
          return module;
        },
        (error: unknown) => {
          loading = null;
          throw error;
        },
      );
      return loading;
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}

/** 読み込み済みなら返し、読み込み終わったら描き直す。読み込みは始めない。 */
export function useLoaded<T>(loader: LazyLoader<T>): T | null {
  return useSyncExternalStore(loader.subscribe, loader.loaded, loader.loaded);
}

/**
 * 表示されたときに読み込みを始め（when が false なら始めない）、読み込み終わったら描き直す。読み込むまでは null。
 * 読み込めなかったときは onError を呼び、null のままにする（読み込み前の表示が残る）。
 */
export function useLazy<T>(loader: LazyLoader<T>, onError?: (error: unknown) => void, when = true): T | null {
  const module = useLoaded(loader);
  useEffect(() => {
    if (module || !when) return;
    loader.load().catch((error: unknown) => onError?.(error));
  }, [loader, module, onError, when]);
  return module;
}
