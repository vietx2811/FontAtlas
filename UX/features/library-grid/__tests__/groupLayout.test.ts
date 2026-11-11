import { describe, expect, it } from 'vitest';
import { HEADER_H, buildLayout, faceSpansOf, groupsCover, styleCountParts, type FamilyGroup } from '../groupLayout';

const g = (family: string, start: number, count: number, matchCount = count, styleTotal = count): FamilyGroup => ({ family, start, count, matchCount, styleTotal });
const G = [g('Alpha', 0, 5), g('Beta', 5, 1), g('Gamma', 6, 4)];

describe('buildLayout', () => {
  it('one header per group, then rows of `cols` cards; the last row of a group is not filled from the next group', () => {
    const l = buildLayout(G, 4, new Set());
    expect(l).toEqual([
      { kind: 'head', g: 0 }, { kind: 'cards', g: 0, first: 0, n: 4 }, { kind: 'cards', g: 0, first: 4, n: 1 },
      { kind: 'head', g: 1 }, { kind: 'cards', g: 1, first: 5, n: 1 },
      { kind: 'head', g: 2 }, { kind: 'cards', g: 2, first: 6, n: 4 },
    ]);
  });
  it('a collapsed group keeps only its header; other groups keep their face indices', () => {
    const l = buildLayout(G, 4, new Set(['Alpha']));
    expect(l[0]).toEqual({ kind: 'head', g: 0 });
    expect(l[1]).toEqual({ kind: 'head', g: 1 });
    expect(l.filter(r => r.kind === 'cards')).toEqual([{ kind: 'cards', g: 1, first: 5, n: 1 }, { kind: 'cards', g: 2, first: 6, n: 4 }]);
  });
  it('every face appears exactly once across the card rows when nothing is collapsed (any column count)', () => {
    for (const cols of [1, 2, 3, 7, 20]) {
      const seen: number[] = [];
      for (const r of buildLayout(G, cols, new Set())) if (r.kind === 'cards') for (let k = 0; k < r.n; k++) seen.push(r.first + k);
      expect(seen).toEqual(Array.from({ length: 10 }, (_, i) => i));
    }
  });
  it('cols < 1 is treated as 1; no groups → no rows', () => {
    expect(buildLayout(G, 0, new Set()).filter(r => r.kind === 'cards')).toHaveLength(10);
    expect(buildLayout([], 4, new Set())).toEqual([]);
  });
});

describe('groupsCover', () => {
  it('only exact, contiguous cover of the total is usable', () => {
    expect(groupsCover(G, 10)).toBe(true);
    expect(groupsCover(G, 11)).toBe(false);
    expect(groupsCover(G, 9)).toBe(false);
    expect(groupsCover(null, 10)).toBe(false);
    expect(groupsCover(G, null)).toBe(false);
    expect(groupsCover([], 0)).toBe(false);
    expect(groupsCover([g('A', 0, 2), g('B', 3, 2)], 5)).toBe(false); // lỗ hổng
    expect(groupsCover([g('A', 0, 0)], 0)).toBe(false);
  });
});

describe('labels and spans', () => {
  it('style count: filtered only when matches < total', () => {
    expect(styleCountParts(g('A', 0, 4, 4, 9))).toEqual({ shown: 4, total: 9, filtered: true });
    expect(styleCountParts(g('A', 0, 9, 9, 9)).filtered).toBe(false);
  });
  it('face spans: a header asks for its representative face, a card row for its cards', () => {
    const l = buildLayout(G, 4, new Set());
    expect(faceSpansOf(l, G, 0, 4)).toEqual([[0, 0], [0, 3], [4, 4], [5, 5], [5, 5]]);
    expect(faceSpansOf(l, G, 90, 99)).toEqual([]);
  });
  it('header height is a constant (virtualised rowHeight is a function of the row index)', () => { expect(HEADER_H).toBe(32); });
});
