// サイトの外へのリンク（ADR 0057）。新しいタブで開く。

/** 新しいタブで開くための属性。開いた先のページから、このページを操作させない（noopener）。 */
export const EXTERNAL_LINK_PROPS = { target: "_blank", rel: "noopener" } as const;

/**
 * サイトの外へのリンクか。http か https で、このページと違うオリジンのものだけを外とみなす。
 * サイト内のパス、ページ内の #…、mailto: などは外とみなさない。オリジンが分からない（サーバーでの描画）ときは、http と https をすべて外とみなす。
 */
export function isExternal(href: unknown, origin: string | undefined = globalThis.location?.origin): boolean {
  if (typeof href !== "string" || !/^https?:\/\//i.test(href)) return false;
  try {
    return origin === undefined || new URL(href).origin !== origin;
  } catch {
    return false;
  }
}
