import { describe, expect, it } from 'vitest';
import { FAMILIES, LATIN, SYSTEM_FAMILIES, VIET, installedFamilies, isInstalled, pickBlocks, rerollBlocks } from '../glyphs';

const hasMark = (c: string) => VIET.includes(c);

describe('pickBlocks (seeded)', () => {
  it('is deterministic for a seed and differs between seeds', () => {
    expect(pickBlocks(42, 9)).toEqual(pickBlocks(42, 9));
    expect(pickBlocks(42, 9)).not.toEqual(pickBlocks(43, 9));
  });
  it('returns the requested count (6–9), each glyph from the Latin or Vietnamese set and each font from the system list', () => {
    for (const n of [6, 7, 8, 9]) {
      const b = pickBlocks(7, n);
      expect(b).toHaveLength(n);
      for (const x of b) { expect(LATIN + VIET).toContain(x.char); expect(SYSTEM_FAMILIES).toContain(x.family); }
    }
  });
  it('mixes plain Latin with Vietnamese letters carrying marks', () => {
    for (let seed = 1; seed <= 50; seed++) {
      const b = pickBlocks(seed, 9);
      expect(b.filter(x => hasMark(x.char)).length, `seed ${seed}`).toBeGreaterThanOrEqual(3);
      expect(b.filter(x => !hasMark(x.char)).length, `seed ${seed}`).toBeGreaterThanOrEqual(3);
    }
  });
  it('never puts the same font on two neighbouring blocks', () => {
    for (let seed = 1; seed <= 50; seed++) {
      const b = pickBlocks(seed, 9);
      for (let i = 1; i < b.length; i++) expect(b[i].family, `seed ${seed} @${i}`).not.toBe(b[i - 1].family);
    }
  });
  it('gives each block a rotation delay so they do not all turn in lockstep', () => {
    const b = pickBlocks(5, 9);
    expect(new Set(b.map(x => x.delayMs)).size).toBeGreaterThan(4);
    for (const x of b) { expect(x.delayMs).toBeGreaterThanOrEqual(0); expect(x.delayMs).toBeLessThan(1200); }
  });
});

describe('rerollBlocks (one beat)', () => {
  it('changes a few blocks and keeps the rest, deterministically per (seed, tick)', () => {
    const base = pickBlocks(9, 9);
    const next = rerollBlocks(base, 9, 1);
    expect(next).toEqual(rerollBlocks(base, 9, 1));
    const changed = next.filter((x, i) => x.char !== base[i].char || x.family !== base[i].family).length;
    expect(changed).toBeGreaterThanOrEqual(2);
    expect(changed).toBeLessThanOrEqual(4);
    expect(rerollBlocks(base, 9, 2)).not.toEqual(next);
  });
  it('keeps every invariant after many beats', () => {
    let b = pickBlocks(3, 9);
    for (let t = 1; t <= 60; t++) {
      b = rerollBlocks(b, 3, t);
      expect(b).toHaveLength(9);
      for (let i = 1; i < b.length; i++) expect(b[i].family).not.toBe(b[i - 1].family);
      for (const x of b) { expect(SYSTEM_FAMILIES).toContain(x.family); expect(LATIN + VIET).toContain(x.char); }
    }
  });
});

const kindOf = (name: string) => FAMILIES.find(f => f.name === name)!.kind;
const uses = (b: { family: string }[]) => b.reduce<Record<string, number>>((m, x) => ({ ...m, [x.family]: (m[x.family] ?? 0) + 1 }), {});

describe('font choice: only fonts that really exist, varied, not repeated', () => {
  it('no family is used more than twice among 9 blocks, for many seeds and after many beats', () => {
    for (let seed = 1; seed <= 100; seed++) {
      let b = pickBlocks(seed, 9);
      expect(Math.max(...Object.values(uses(b))), `seed ${seed}`).toBeLessThanOrEqual(2);
      for (let t = 1; t <= 20; t++) { b = rerollBlocks(b, seed, t); expect(Math.max(...Object.values(uses(b))), `seed ${seed} tick ${t}`).toBeLessThanOrEqual(2); }
    }
  });

  it('mixes genres (serif, sans, mono, script, display): at least 3 different kinds among 9 blocks', () => {
    for (let seed = 1; seed <= 100; seed++) {
      const kinds = new Set(pickBlocks(seed, 9).map(x => kindOf(x.family)));
      expect(kinds.size, `seed ${seed}`).toBeGreaterThanOrEqual(3);
    }
  });

  it('Vietnamese letters are only set in fonts that carry them (script/display fonts get plain Latin)', () => {
    for (let seed = 1; seed <= 100; seed++) {
      for (const b of pickBlocks(seed, 9)) {
        if (VIET.includes(b.char)) expect(FAMILIES.find(f => f.name === b.family)!.viet, `${b.family} for ${b.char}`).toBe(true);
      }
    }
  });

  it('draws only from the families the machine really has', () => {
    const have = ['Helvetica Neue', 'Georgia', 'Menlo', 'Didot', 'Avenir Next', 'Palatino'];
    for (let seed = 1; seed <= 50; seed++) {
      let b = pickBlocks(seed, 9, have);
      for (const x of b) expect(have).toContain(x.family);
      for (let t = 1; t <= 10; t++) { b = rerollBlocks(b, seed, t, have); for (const x of b) expect(have).toContain(x.family); }
    }
  });

  it('with only a few fonts installed it still fills 9 blocks (repeats allowed only when it must)', () => {
    const b = pickBlocks(3, 9, ['Helvetica Neue', 'Georgia']);
    expect(b).toHaveLength(9);
    expect(b.every(x => ['Helvetica Neue', 'Georgia'].includes(x.family))).toBe(true);
  });

  it('exposes the family names as a flat list too', () => {
    expect(SYSTEM_FAMILIES).toEqual(FAMILIES.map(f => f.name));
    expect(new Set(FAMILIES.map(f => f.kind))).toEqual(new Set(['serif', 'sans', 'mono', 'script', 'display']));
  });
});

describe('detecting installed fonts by width (the font-detect trick: a missing font falls back, so every width equals the fallback width)', () => {
  const widths: Record<string, number> = { monospace: 100, serif: 120, 'sans-serif': 110 };
  /** Giả lập đo chữ: font có thật cho độ rộng riêng; font thiếu rơi về họ dự phòng đứng sau. */
  const measure = (installed: string[]) => (font: string) => {
    const m = /^"(.+)", (monospace|serif|sans-serif)$/.exec(font);
    if (!m) return widths[font] ?? 0;
    return installed.includes(m[1]) ? 150 + m[1].length : widths[m[2]];
  };
  it('a font that changes the width against some fallback is installed', () => {
    expect(isInstalled('Georgia', measure(['Georgia']))).toBe(true);
    expect(isInstalled('Papyrus', measure(['Georgia']))).toBe(false);
  });
  it('filters the candidate list', () => {
    const got = installedFamilies(FAMILIES, measure(['Georgia', 'Menlo', 'Didot']));
    expect(got).toEqual(['Georgia', 'Didot', 'Menlo']); // theo thứ tự của danh sách ứng viên
  });
  it('without a measuring canvas nothing can be tested, so every family is assumed available', () => {
    expect(installedFamilies(FAMILIES, null)).toEqual(FAMILIES.map(f => f.name));
  });
});
