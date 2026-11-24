// Mọi nơi gọi *Subscribe phải có tên subscriber CỐ ĐỊNH (chuỗi hằng) và không trùng nhau:
// backend khoá sink theo (webview, kênh, subscriber) nên hai nơi cùng tên (hoặc cùng "default") sẽ đá nhau → lưới trống, số đếm đứng.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const ROOT = join(__dirname, '..', '..');
const SKIP = new Set(['node_modules', '__tests__', 'bindings', 'dist', 'src-tauri', 'test', 'api']);
const walk = (dir: string): string[] => readdirSync(dir).flatMap(n => {
  const p = join(dir, n); const st = statSync(p);
  if (st.isDirectory()) return SKIP.has(n) ? [] : walk(p);
  return /\.(ts|tsx)$/.test(n) ? [p] : [];
});
/** Tìm `api.xxxSubscribe(` rồi lấy đối số cuối của lời gọi (đếm ngoặc). */
const sites = () => walk(ROOT).flatMap(f => {
  const s = readFileSync(f, 'utf8'); const out: Array<{ file: string; kind: string; last: string }> = [];
  for (const m of s.matchAll(/\bapi\.(boot|scan|embed|activation|task)Subscribe\(/g)) {
    let d = 0; let i = m.index! + m[0].length - 1; const start = i;
    for (; i < s.length; i++) { if (s[i] === '(') d++; else if (s[i] === ')') { d--; if (d === 0) break; } }
    const call = s.slice(start + 1, i); const mm = call.match(/,\s*'([a-z][a-z0-9.]*)'\s*$/);
    out.push({ file: f.slice(ROOT.length), kind: m[1], last: mm ? mm[1] : '' });
  }
  return out;
});

describe('tên subscriber', () => {
  it('mỗi lời gọi có tên chuỗi hằng', () => {
    const all = sites();
    expect(all.length).toBeGreaterThanOrEqual(7);
    for (const s of all) expect(s.last, `${s.file} ${s.kind}Subscribe thiếu tên subscriber`).not.toBe('');
  });
  it('không trùng nhau trong cùng một kênh', () => {
    const seen = new Set<string>();
    for (const s of sites()) { const k = `${s.kind}:${s.last}`; expect(seen.has(k), `trùng ${k}`).toBe(false); seen.add(k); }
  });
});
