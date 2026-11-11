import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { describe, expect, it } from 'vitest';

/**
 * Chặn chuỗi tiếng Việt viết cứng trong mã giao diện (JSX, aria-label, title, thông báo…): chuỗi người dùng thấy phải đi qua `t()`.
 * Quét .ts/.tsx ngoài test, i18n, dữ liệu mẫu; bỏ comment. Baseline = số dòng còn chữ Việt của từng file; chỉ được GIẢM.
 * Chuyển một file xong thì chạy `UPDATE_I18N_BASELINE=1 npx vitest run i18n/__tests__/hardcoded.test.ts` để hạ baseline.
 */
const ROOT = join(__dirname, '../..');
const BASELINE = join(__dirname, 'hardcodedBaseline.json');
const SKIP_DIRS = new Set(['node_modules', 'src-tauri', 'bindings', '__tests__', 'i18n', 'test', 'dist', 'public']);
// Dữ liệu mẫu/fixture, không phải chuỗi giao diện.
const SKIP_FILE = /(^|\/)(api\/mock[A-Za-z]*\.ts|store\/devPreset\.ts|features\/library-grid\/perf\/.*|components\/onboarding\/glyphOutlines\.ts|vite-env\.d\.ts)$/;
const VN = /[àáảãạăằắẳẵặâầấẩẫậèéẻẽẹêềếểễệìíỉĩịòóỏõọôồốổỗộơờớởỡợùúủũụưừứửữựỳýỷỹỵđÀÁẢÃẠĂẰẮẲẴẶÂẦẤẨẪẬÈÉẺẼẸÊỀẾỂỄỆÌÍỈĨỊÒÓỎÕỌÔỒỐỔỖỘƠỜỚỞỠỢÙÚỦŨỤƯỪỨỬỮỰỲÝỶỸỴĐ]/;

export function stripComments(s: string): string {
  let out = ''; let i = 0; let q: string | null = null;
  while (i < s.length) {
    const c = s[i];
    if (q === null) {
      if (s.startsWith('//', i)) { const j = s.indexOf('\n', i); i = j < 0 ? s.length : j; continue; }
      if (s.startsWith('/*', i)) { const j = s.indexOf('*/', i + 2); const end = j < 0 ? s.length : j + 2; out += s.slice(i, end).replace(/[^\n]/g, ''); i = end; continue; }
      if (c === '\'' || c === '"' || c === '`') q = c;
      out += c; i++;
    } else {
      if (c === '\\') { out += s.slice(i, i + 2); i += 2; continue; }
      if (c === q) q = null;
      out += c; i++;
    }
  }
  return out;
}
/** Dòng nào có `i18n-ok` (chữ mẫu, dữ liệu, regex gập dấu…) là chủ ý, không phải chuỗi giao diện. */
export const vietnameseLines = (src: string): number => {
  const ok = new Set(src.split('\n').flatMap((l, i) => (l.includes('i18n-ok') ? [i] : [])));
  // `glyph="ạ"` / `glyph: 'ụ'` là chữ minh hoạ trên trạng thái trống, không phải câu chữ.
  return stripComments(src).replace(/glyph[:=]\s*\{?(['"`]).\1\}?/g, '').split('\n').filter((l, i) => VN.test(l) && !ok.has(i)).length;
};

function walk(dir: string, acc: string[] = []): string[] {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    if (e.isDirectory()) { if (!SKIP_DIRS.has(e.name)) walk(join(dir, e.name), acc); }
    else if (/\.(ts|tsx)$/.test(e.name) && !e.name.endsWith('.d.ts')) acc.push(join(dir, e.name));
  }
  return acc;
}
const scan = (): Record<string, number> => {
  const res: Record<string, number> = {};
  for (const f of walk(ROOT)) {
    const rel = relative(ROOT, f).split('\\').join('/');
    if (SKIP_FILE.test(rel)) continue;
    const n = vietnameseLines(readFileSync(f, 'utf8'));
    if (n > 0) res[rel] = n;
  }
  return res;
};

describe('no hard-coded Vietnamese in UI code', () => {
  it('comments are ignored, strings and JSX text are counted', () => {
    expect(vietnameseLines('// Tiếng Việt\n/* Thư viện */\nconst a = 1;')).toBe(0);
    expect(vietnameseLines('const a = "Thư viện";')).toBe(1);
    expect(vietnameseLines('return <b>Cài đặt</b>; // ghi chú')).toBe(1);
    expect(vietnameseLines('const s = "Ăn phở"; // i18n-ok: chữ mẫu')).toBe(0);
  });
  it('no file gains Vietnamese lines beyond its baseline (convert to t() and lower the baseline)', () => {
    const now = scan();
    if (process.env.UPDATE_I18N_BASELINE) { writeFileSync(BASELINE, JSON.stringify(Object.fromEntries(Object.entries(now).sort()), null, 1) + '\n'); return; }
    const base = JSON.parse(readFileSync(BASELINE, 'utf8')) as Record<string, number>;
    const worse = Object.entries(now).filter(([f, n]) => n > (base[f] ?? 0)).map(([f, n]) => `${f}: ${n} lines (baseline ${base[f] ?? 0})`);
    expect(worse, 'Vietnamese strings outside i18n — use t() with an en translation').toEqual([]);
    const stale = Object.keys(base).filter(f => !(f in now)).map(f => `${f} is clean now; remove it from the baseline`);
    expect(stale).toEqual([]);
  });
});
