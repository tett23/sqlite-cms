/** ページの meta description。ページ自身の説明（article の要約）があればそれを、なければサイトの説明を使う。 */
export function pageDescription(siteDescription: string, ownDescription?: string | null): string {
  return ownDescription?.trim() ? ownDescription : siteDescription;
}

export function setMetaDescription(doc: Document, content: string) {
  let meta = doc.head.querySelector<HTMLMetaElement>('meta[name="description"]');
  if (!meta) {
    meta = doc.createElement("meta");
    meta.name = "description";
    doc.head.appendChild(meta);
  }
  meta.content = content;
}
