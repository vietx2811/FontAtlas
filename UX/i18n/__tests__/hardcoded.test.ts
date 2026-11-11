import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import ts from 'typescript';
import { describe, expect, it } from 'vitest';

/**
 * Chặn chuỗi giao diện viết cứng, quét bằng AST (không phải regex theo dòng):
 *  - mọi chuỗi (literal, template, JSX text) có chữ Việt có dấu;
 *  - trong .tsx: JSX text và thuộc tính aria-label / title / placeholder / alt viết thẳng bằng chữ (kể cả tiếng Anh, tiếng Việt không dấu).
 * Chuỗi hợp lệ (chữ mẫu, tên riêng, ký hiệu) phải nằm trong ALLOWED với đúng file và đúng nội dung — không miễn theo dòng.
 * Baseline = số vi phạm của từng file; chỉ được GIẢM. Hạ baseline: `UPDATE_I18N_BASELINE=1 npx vitest run i18n/__tests__/hardcoded.test.ts`.
 */
const ROOT = join(__dirname, '../..');
const BASELINE = join(__dirname, 'hardcodedBaseline.json');
// Chỉ các đường dẫn cụ thể (không bỏ qua theo tên thư mục ở mọi cấp).
const SKIP = [/^node_modules\//, /^src-tauri\//, /^bindings\//, /^dist\//, /^public\//, /^test\//, /^i18n\//, /(^|\/)__tests__\//, /^api\/mock[A-Za-z]*\.ts$/, /^store\/devPreset\.ts$/, /^features\/library-grid\/perf\//, /^components\/onboarding\/glyphOutlines\.ts$/, /\.d\.ts$/];
const VN = /[àáảãạăằắẳẵặâầấẩẫậèéẻẽẹêềếểễệìíỉĩịòóỏõọôồốổỗộơờớởỡợùúủũụưừứửữựỳýỷỹỵđÀÁẢÃẠĂẰẮẲẴẶÂẦẤẨẪẬÈÉẺẼẸÊỀẾỂỄỆÌÍỈĨỊÒÓỎÕỌÔỒỐỔỖỘƠỜỚỞỠỢÙÚỦŨỤƯỪỨỬỮỰỲÝỶỸỴĐ]/;
const LETTERS = /[A-Za-zÀ-ỹ]{2,}/;
const ATTRS = new Set(['aria-label', 'title', 'placeholder', 'alt']);
/** `file::nội dung` chính xác: chữ mẫu, tên riêng, dữ liệu, không phải câu chữ giao diện. */
const ALLOWED = new Set<string>([
  'components/settings/AutoActivation.tsx::Auto-activation', 'components/activation/ActivationDialogs.tsx::persistent scope', 'components/activation/ActivationDialogs.tsx::session scope', // thuật ngữ font giữ tiếng Anh, có TermTip + gloss
  'components/compare/CompareCells.tsx::Hxgă', 'components/compare/CompareCells.tsx::ỮẶỆỖỞ ươ',
  'components/detail/tabs/SpecimenTab.tsx::Hà Nội mùa thu, cây cơm nguội vàng, cây bàng lá đỏ, nằm kề bên nhau, phố xưa nhà cổ, mái ngói thâm nâu.',
  'components/detail/tabs/SpecimenTab.tsx::Đẹp như Hà Nội', 'i18n/sample.ts::Chữ Việt đẹp — Ấn tượng', 'i18n/sample.ts::Typeface · Chữ Việt', 'i18n/sample.ts::Phở bò Hà Nội',
  'components/main/Library.tsx::Ặa', 'features/library-grid/types.ts::Hamburgefonts Ăn phở',
  'components/main/SampleComposer.tsx::Ặẫ', 'components/main/SampleComposer.tsx::Ặẫỡự',
  'components/settings/Settings.tsx::Phở bò Hà Nội', 'components/settings/Settings.tsx::ẮẰẲẴẶ ẤẦẨẪẬ ƠỚỜỞỠỢ ƯỨỪỬỮỰ', 'components/settings/Settings.tsx::Cà phê sữa đá 35.000đ', 'components/settings/Settings.tsx::Hà Nội mùa thu, cây cơm nguội vàng',
  'components/detail/tabs/ViTab.tsx::àáảãạăằắẳẵặâầấẩẫậèéẻẽẹêềếểễệìíỉĩịòóỏõọôồốổỗộơờớởỡợùúủũụưừứửữựỳýỷỹỵđ', 'components/detail/tabs/ViTab.tsx::Ắ Ằ Ẳ Ẵ Ặ', 'components/detail/tabs/ViTab.tsx::ấ ầ ẩ ẫ ậ', 'components/detail/tabs/ViTab.tsx::Ơ ơ Ư ư Đ đ',
  'components/detail/DetailPanel.tsx::Ặ', 'components/main/TopBar.tsx::Ặ vi', 'features/library-grid/LangBadge.tsx::vie',
  'features/library-grid/api.ts::đ',
]);
// Từ tiếng Anh/ký hiệu hợp lệ viết thẳng (tên sản phẩm, đơn vị, định dạng).
const NEUTRAL = new Set(['Hxg', 'stacked_marks_overflow', 'dataless', 'Always keep on this device', 'VAR', 'Tiếng Việt', 'English', 'Font Atlas', 'Aa', 'px', 'MB', 'GB', 'CPU', 'GPU', 'Metal', 'PNG', 'PDF', 'OTF', 'TTF', 'TTC', 'WOFF', 'WOFF2', 'OFL', 'Apache', 'Variable', 'Italic', 'Hamburgefonts']);

/** In danh sách vi phạm (dùng khi chuyển chuỗi): `LIST_I18N=1 npx vitest run i18n/__tests__/hardcoded.test.ts`. */
export function violations(file: string, src: string): string[] {
  const sf = ts.createSourceFile(file, src, ts.ScriptTarget.Latest, true, file.endsWith('.tsx') ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
  const out: string[] = [];
  const tsx = file.endsWith('.tsx');
  const add = (text: string, node: ts.Node) => {
    const clean = text.replace(/\s+/g, ' ').trim();
    if (!clean || ALLOWED.has(`${file}::${clean}`)) return;
    out.push(`${file}:${sf.getLineAndCharacterOfPosition(node.getStart(sf)).line + 1} ${clean.slice(0, 60)}`);
  };
  const inAttr = (n: ts.Node): string | null => {
    for (let p: ts.Node | undefined = n.parent; p; p = p.parent) { if (ts.isJsxAttribute(p)) return ts.isIdentifier(p.name) ? p.name.text : p.name.getText(sf); if (ts.isCallExpression(p) || ts.isJsxElement(p) || ts.isBlock(p)) return null; }
    return null;
  };
  const visit = (n: ts.Node): void => {
    if (ts.isStringLiteral(n) || ts.isNoSubstitutionTemplateLiteral(n)) {
      if (ts.isImportDeclaration(n.parent) || ts.isExportDeclaration(n.parent)) return;
      if (VN.test(n.text)) { if (n.text.length > 1 && !NEUTRAL.has(n.text)) add(n.text, n); } // 1 ký tự = glyph minh hoạ (ZeroState…)
      else if (tsx) { const a = inAttr(n); if (a && ATTRS.has(a) && LETTERS.test(n.text) && !NEUTRAL.has(n.text)) add(n.text, n); }
    } else if (ts.isTemplateHead(n) || ts.isTemplateMiddle(n) || ts.isTemplateTail(n)) { if (VN.test(n.text)) add(n.text, n); }
    else if (ts.isJsxText(n) && tsx) { if (VN.test(n.text) || (LETTERS.test(n.text) && !NEUTRAL.has(n.text.trim()))) add(n.text, n); }
    ts.forEachChild(n, visit);
  };
  visit(sf);
  return out;
}

function walk(dir: string, acc: string[] = []): string[] {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, e.name); const rel = relative(ROOT, p).split('\\').join('/');
    if (e.isDirectory()) { if (!SKIP.some(r => r.test(`${rel}/`))) walk(p, acc); }
    else if (/\.(ts|tsx)$/.test(e.name) && !SKIP.some(r => r.test(rel))) acc.push(p);
  }
  return acc;
}
const scan = (): Record<string, string[]> => {
  const res: Record<string, string[]> = {};
  for (const f of walk(ROOT)) {
    const rel = relative(ROOT, f).split('\\').join('/');
    const v = violations(rel, readFileSync(f, 'utf8'));
    if (v.length) res[rel] = v;
  }
  return res;
};

describe('no hard-coded UI strings', () => {
  it('JSX text, string literals with Vietnamese and aria-label/title/placeholder/alt are detected; comments and t() calls are not', () => {
    expect(violations('a.tsx', '// Tiếng Việt\n/* Thư viện */\nconst a = 1;')).toEqual([]);
    expect(violations('a.tsx', 'const a = "Thư viện";')).toHaveLength(1);
    expect(violations('a.tsx', 'const a = <b>Cài đặt</b>;')).toHaveLength(1);
    expect(violations('a.tsx', 'const a = <b aria-label="Trang sau">x</b>;')).toHaveLength(1);
    expect(violations('a.tsx', 'const a = <b>Next page</b>;')).toHaveLength(1);
    expect(violations('a.tsx', "const a = <b aria-label={t('x')} title={`${n} MB`}>{t('y')}</b>;")).toEqual([]);
    expect(violations('a.tsx', 'const a = <b>×</b>;')).toEqual([]);
    expect(violations('components/main/Library.tsx', 'const a = "Ặa";')).toEqual([]);
    expect(violations('a.ts', 'const a = "Ặa"; // i18n-ok')).toHaveLength(1); // không miễn theo dòng (chỉ ALLOWED theo file + nội dung)
  });
  it('lists violations on request', () => { if (process.env.LIST_I18N) console.log(Object.values(scan()).flat().join('\n')); });
  it('no file gains violations beyond its baseline (convert to t() and lower the baseline)', () => {
    const now = scan();
    const counts = Object.fromEntries(Object.entries(now).map(([f, v]) => [f, v.length]).sort());
    if (process.env.UPDATE_I18N_BASELINE) { writeFileSync(BASELINE, JSON.stringify(counts, null, 1) + '\n'); return; }
    const base = JSON.parse(readFileSync(BASELINE, 'utf8')) as Record<string, number>;
    const worse = Object.entries(now).filter(([f, v]) => v.length > (base[f] ?? 0)).map(([f, v]) => `${f}: ${v.length} (baseline ${base[f] ?? 0})\n  ${v.slice(0, 6).join('\n  ')}`);
    expect(worse, 'UI strings outside i18n — use t() with an en translation').toEqual([]);
    expect(Object.keys(base).filter(f => !(f in now)).map(f => `${f} is clean now; remove it from the baseline`)).toEqual([]);
  });
});
