// Thuần hàm: gom font dùng trong một danh sách node text (Figma TextNode hoặc mock).
// Không gọi mạng. Kết quả đi sang app qua URL/clipboard.
function collectFonts(textNodes, availableKeys /* Set "family|style" */) {
  const seen = new Map();
  for (const n of textNodes) {
    const names = n.getRangeAllFontNames(0, n.characters.length);
    for (const f of names) {
      const k = f.family + '|' + f.style;
      const e = seen.get(k) || { family: f.family, style: f.style, nodes: 0, missing: false };
      e.nodes += 1;
      // hasMissingFont chỉ nói CẢ node có font thiếu; đối chiếu với danh sách khả dụng để biết font nào.
      if (n.hasMissingFont && availableKeys && !availableKeys.has(k)) e.missing = true;
      seen.set(k, e);
    }
  }
  return [...seen.values()].sort((a, b) => a.family.localeCompare(b.family) || a.style.localeCompare(b.style));
}
// base64url(JSON) cho fontatlas://fonts?v=1&d=...
function toLink(fonts, fileName, scheme = 'fontatlas') {
  const json = JSON.stringify({ v: 1, src: 'figma', file: fileName, fonts: fonts.map(f => [f.family, f.style, f.missing ? 1 : 0]) });
  const b64 = Buffer.from(json, 'utf8').toString('base64').replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  return scheme + '://fonts?v=1&d=' + b64;
}
module.exports = { collectFonts, toLink };
