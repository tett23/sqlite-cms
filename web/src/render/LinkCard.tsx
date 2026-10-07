// リンクカード（ADR 0028、0049）。本体とは別のチャンクにして、URL だけの行が画面の近くに来てから読み込む。
import { EXTERNAL_LINK_PROPS, isExternal } from "../externalLink";
import { LINK_CARD_CLASS, LINK_CARD_HOST_CLASS, LINK_CARD_IMAGE_CLASS, LINK_CARD_TEXT_CLASS, LINK_CARD_URL_CLASS } from "../markdown/transforms";

/**
 * URL のカード。ホスト名と URL を表示する。
 * 画像は、CLI がビルドのときにそのページから取得し、カードの大きさの WebP にして配信するもの（ADR 0049）。
 * 画像はリンクの文字列（ホスト名と URL）を補う飾りなので、代替テキストは空にする。
 */
export function LinkCard({ url, image }: { url: string; image?: string }) {
  return (
    <a href={url} className={LINK_CARD_CLASS} {...(isExternal(url) ? EXTERNAL_LINK_PROPS : {})}>
      <span className={LINK_CARD_TEXT_CLASS}>
        <span className={LINK_CARD_HOST_CLASS}>{new URL(url).host}</span>
        <span className={LINK_CARD_URL_CLASS}>{url}</span>
      </span>
      {image && <img src={image} alt="" loading="lazy" className={LINK_CARD_IMAGE_CLASS} />}
    </a>
  );
}
