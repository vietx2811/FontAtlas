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
  const dark = { '--sheet': '#0C0A09', '--card': '#151311', '--panel': '#1C1917', '--float': '#292522', '--fg': '#FAFAF9', '--muted': '#A8A29E', '--faint': '#8A837D', '--surface-2': '#24211E', '--accent': '#E0B07E', '--accent-ink': '#E0B07E', '--on': '#4ADE80', '--warn': '#FB923C' };
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

  it('dark tiers are ordered canvas < card < panel < float and the border is a hairline', () => {
    const forced = block(/:root\[data-theme="dark"\][^{]*{([^}]*)}/);
    const lum = (k: string) => { const h = forced.match(new RegExp(`${k}:\\s*#([0-9a-f]{6})`, 'i'))![1]; return parseInt(h.slice(0, 2), 16) + parseInt(h.slice(2, 4), 16) + parseInt(h.slice(4), 16); };
    expect(lum('--sheet')).toBeLessThan(lum('--card'));
    expect(lum('--card')).toBeLessThan(lum('--panel'));
    expect(lum('--panel')).toBeLessThan(lum('--float'));
    expect(forced).toMatch(/--border:\s*rgba\(250, 250, 249, \.0[5-9]\)/);
    expect(forced).toMatch(/--specimen-bg:\s*#0A0908/i);
  });

  it('keeps the old names working as aliases', () => {
    const b = block(/\n:root\s*{([^}]*)}/);
    expect(b).toMatch(/--foreground:\s*var\(--fg\);/);
    expect(b).toMatch(/--background:\s*var\(--sheet\);/);
  });
});

describe('dark --faint keeps 4.5:1 on canvas and panel', () => {
  const lum = (hex: string) => { const v = [1, 3, 5].map(i => parseInt(hex.slice(i, i + 2), 16) / 255).map(c => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4)); return 0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2]; };
  const ratio = (a: string, b: string) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); };
  it('against --sheet and --panel', () => {
    const forced = css.match(/:root\[data-theme="dark"\][^{]*{([^}]*)}/)![1];
    const get = (k: string) => forced.match(new RegExp(`${k}:\\s*(#[0-9a-f]{6})`, 'i'))![1];
    expect(ratio(get('--faint'), get('--sheet'))).toBeGreaterThanOrEqual(4.5);
    expect(ratio(get('--faint'), get('--panel'))).toBeGreaterThanOrEqual(4.5);
  });
});

describe('NoteMark is red in both themes', () => {
  const lum = (hex: string) => { const v = [1, 3, 5].map(i => parseInt(hex.slice(i, i + 2), 16) / 255).map(c => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4)); return 0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2]; };
  const ratio = (a: string, b: string) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + 0.05) / (y + 0.05); };
  const red = (h: string) => { const [r, g, b] = [1, 3, 5].map(i => parseInt(h.slice(i, i + 2), 16)); return r > 180 && r > g * 1.6 && r > b * 1.6; };
  it('dark: its own red token (not the orange --warn) with ≥ 3:1 on the canvas and a readable glyph on it', () => {
    const forced = css.match(/:root\[data-theme="dark"\][^{]*{([^}]*)}/)![1];
    const get = (k: string) => forced.match(new RegExp(`${k}:\\s*(#[0-9a-f]{6})`, 'i'))![1];
    expect(red(get('--note'))).toBe(true);
    expect(get('--note')).not.toBe(get('--warn'));
    expect(ratio(get('--note'), get('--sheet'))).toBeGreaterThanOrEqual(3);
    expect(ratio(get('--note-ink'), get('--note'))).toBeGreaterThanOrEqual(4.5);
  });
  it('the OS-dark block carries the same token', () => {
    expect(css.match(/--note: #F87171/g)!.length).toBe(2);
  });
  it('light keeps a red token with a white glyph', () => {
    const root = css.match(/:root\s*{([^}]*)}/)![1];
    expect(root).toMatch(/--note: #DC2626/);
    expect(ratio('#DC2626', '#FFFFFF')).toBeGreaterThanOrEqual(4.5);
  });
});
