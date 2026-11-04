import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const UX = resolve(__dirname, '../../..');
const css = readFileSync(join(UX, 'index.css'), 'utf8');
const SKIP = new Set(['node_modules', 'dist', 'target', '.vite']);
// Ghép chuỗi để chính file này không chứa tên miền (grep -r trên UX/ phải rỗng).
const BANNED = ['google', 'apis'].join('');

function walk(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    if (SKIP.has(name)) continue;
    const p = join(dir, name);
    const st = statSync(p);
    if (st.isDirectory()) walk(p, out);
    else if (st.size < 5_000_000) out.push(p);
  }
  return out;
}

describe('font UI đóng gói cục bộ (Plan 6b T2, G9)', () => {
  it('no source file in UX/ (outside node_modules, dist, target) references the Google Fonts API', () => {
    const hits = walk(UX).filter(p => /\.(css|ts|tsx|js|html|json|md)$/.test(p) && readFileSync(p, 'utf8').includes(BANNED));
    expect(hits).toEqual([]);
  });

  for (const [family, prefix] of [['Inter', 'inter'], ['JetBrains Mono', 'jetbrains-mono']] as const) {
    it(`${family}: @font-face for latin, latin-ext and vietnamese, files present locally`, () => {
      for (const subset of ['latin', 'latin-ext', 'vietnamese']) {
        const file = `${prefix}-${subset}-wght-normal.woff2`;
        const block = new RegExp(`@font-face\\s*{[^}]*font-family:\\s*"${family}"[^}]*src:\\s*url\\("/fonts/${file}"\\)\\s*format\\("woff2"\\)[^}]*unicode-range:`);
        expect(css).toMatch(block);
        const p = join(UX, 'public/fonts', file);
        expect(existsSync(p), p).toBe(true);
        expect(statSync(p).size).toBeGreaterThan(4_000); // chặn file rỗng; bộ ký tự Việt của JetBrains Mono chỉ ~7,5 KB
      }
    });
  }

  it('ships the OFL license next to the fonts', () => {
    for (const f of ['OFL-Inter.txt', 'OFL-JetBrainsMono.txt']) {
      expect(readFileSync(join(UX, 'public/fonts', f), 'utf8')).toContain('SIL Open Font License, Version 1.1');
    }
  });

  it('UI font stack starts with the bundled Inter', () => {
    expect(css).toMatch(/--font-sans:\s*"Inter",/);
    expect(css).toMatch(/--font-mono-code:\s*"JetBrains Mono",/);
  });
});

describe('token của prototype gộp vào index.css (Plan 6b §4)', () => {
  const light = { '--sheet': '#FAFAFA', '--card': '#FFFFFF', '--fg': '#2A2A2A', '--muted': '#78716C', '--border': '#E7E5E4', '--surface-2': '#F3F2F0', '--accent': '#D4A574', '--accent-ink': '#9A6B3C', '--accent-soft': '#F5E6D3', '--on': '#3FA66B', '--warn': '#C2410C' };
  const dark = { '--sheet': '#1C1917', '--card': '#23201E', '--fg': '#FAFAF9', '--muted': '#A8A29E', '--border': '#2E2A27', '--surface-2': '#2A2623', '--accent': '#E0B07E', '--accent-ink': '#E0B07E', '--on': '#4ADE80', '--warn': '#FB923C' };
  const block = (sel: RegExp) => { const m = css.match(sel); expect(m, String(sel)).toBeTruthy(); return m![1]; };

  it('light values on :root', () => {
    const b = block(/\n:root\s*{([^}]*)}/);
    for (const [k, v] of Object.entries(light)) expect(b, k).toMatch(new RegExp(`${k}:\\s*${v};`, 'i'));
  });

  it('dark values follow the system and can be forced with data-theme', () => {
    const forced = block(/:root\[data-theme="dark"\][^{]*{([^}]*)}/);
    for (const [k, v] of Object.entries(dark)) expect(forced, k).toMatch(new RegExp(`${k}:\\s*${v};`, 'i'));
    expect(css).toMatch(/@media \(prefers-color-scheme: dark\)\s*{\s*:root:not\(\[data-theme="light"\]\)\s*{/);
  });

  it('keeps the old names working as aliases', () => {
    const b = block(/\n:root\s*{([^}]*)}/);
    expect(b).toMatch(/--foreground:\s*var\(--fg\);/);
    expect(b).toMatch(/--background:\s*var\(--sheet\);/);
  });
});
