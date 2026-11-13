import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

// Mảng trái nằm trên kính: nền sau cửa sổ có thể là đen hoặc trắng tuyền. Kiểm bằng số: tint (alpha) trộn với nền xấu nhất, chữ vẫn >= 4.5:1.
const css = readFileSync('components/onboarding/onboarding.css', 'utf8');
const lin = (c: number) => { const s = c / 255; return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4; };
const rgb = (h: string): [number, number, number] => [1, 3, 5].map(i => parseInt(h.slice(i, i + 2), 16)) as [number, number, number];
const lum = ([r, g, b]: [number, number, number]) => 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
const ratio = (a: number, b: number) => (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
const mix = (tint: [number, number, number], alpha: number, back: number): [number, number, number] => tint.map(c => c * alpha + back * (1 - alpha)) as [number, number, number];
const tok = (block: string, name: string) => block.match(new RegExp(`${name}:\\s*(#[0-9A-Fa-f]{6})`))![1];

const light = css.match(/\.fa-onb \{[^}]*--onb-tint:[^}]*\}/)![0];
const dark = css.match(/:root\[data-theme="dark"\] \.fa-onb, \.dark \.fa-onb \{[^}]*\}/)![0];
const alpha = Number(light.match(/--onb-tint-alpha:\s*(\d+)%/)![1]) / 100;

describe.each([['light', light], ['dark', dark]])('onboarding glass contrast (%s)', (_n, block) => {
  const tint = rgb(tok(block, '--onb-tint'));
  it.each(['--onb-fg', '--onb-fg2', '--onb-warn', '--onb-link'])('%s >= 4.5:1 on the worst backdrop (black or white behind the glass)', name => {
    const text = lum(rgb(tok(block, name)));
    for (const back of [0, 255]) expect(ratio(text, lum(mix(tint, alpha, back)))).toBeGreaterThanOrEqual(4.5);
  });
});
it('dark media-query block repeats the explicit dark tokens', () => {
  const m = css.match(/@media \(prefers-color-scheme: dark\) \{\s*:root:not\(\[data-theme="light"\]\) \.fa-onb \{([^}]*)\}/)![1];
  for (const n of ['--onb-tint', '--onb-fg', '--onb-fg2', '--onb-warn', '--onb-link']) expect(m).toContain(`${n}: ${tok(dark, n)}`);
});
