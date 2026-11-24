// Luồng chính của plugin (chạy trong sandbox Figma). Chỉ ĐỌC: không sửa tài liệu, không mạng.
// Build thử: gộp collect.js vào đây (Figma không có require); bản spike giữ tách để test bằng Node.
figma.showUI(__html__, { width: 320, height: 220 });

async function scan() {
  const scope = figma.currentPage.selection.length ? figma.currentPage.selection : figma.currentPage.children;
  const texts = [];
  for (const n of scope) {
    if (n.type === 'TEXT') texts.push(n);
    else if ('findAllWithCriteria' in n) texts.push(...n.findAllWithCriteria({ types: ['TEXT'] }));
  }
  const avail = new Set((await figma.listAvailableFontsAsync()).map(f => f.fontName.family + '|' + f.fontName.style));
  const fonts = collectFonts(texts, avail);
  figma.ui.postMessage({ type: 'fonts', fonts, file: figma.root.name, scope: figma.currentPage.selection.length ? 'selection' : 'page' });
}
figma.ui.onmessage = msg => { if (msg.type === 'scan') scan(); if (msg.type === 'close') figma.closePlugin(); };
scan();
