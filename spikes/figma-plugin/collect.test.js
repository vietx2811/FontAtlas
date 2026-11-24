const assert = require('assert');
const { collectFonts, toLink } = require('./collect');
const mk = (names, missing) => ({ characters: 'abc', hasMissingFont: missing, getRangeAllFontNames: () => names });
const avail = new Set(['Inter|Regular']);
const out = collectFonts([
  mk([{ family: 'Inter', style: 'Regular' }], false),
  mk([{ family: 'Inter', style: 'Regular' }, { family: 'Tiếng Việt Display', style: 'Bold' }], true),
], avail);
assert.deepStrictEqual(out.map(f => [f.family, f.style, f.nodes, f.missing]), [['Inter', 'Regular', 2, false], ['Tiếng Việt Display', 'Bold', 1, true]]);
const link = toLink(out, 'Menu v3');
assert.ok(link.startsWith('fontatlas://fonts?v=1&d='));
const d = JSON.parse(Buffer.from(link.split('d=')[1].replace(/-/g, '+').replace(/_/g, '/'), 'base64').toString('utf8'));
assert.strictEqual(d.fonts.length, 2);
// 60 font ≈ độ dài URL
const many = Array.from({ length: 60 }, (_, i) => ({ family: 'Family Name ' + i, style: 'Semi Bold Italic', missing: i % 3 === 0, nodes: 1 }));
console.log('60 fonts → URL length', toLink(many, 'x').length);
console.log('ok');
