import { describe, expect, it } from 'vitest';
import { LATIN, SYSTEM_FAMILIES, VIET, pickBlocks, rerollBlocks } from '../glyphs';

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
