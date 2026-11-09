import { describe, expect, it } from 'vitest';
import { binOf, classBins, matchesWeight, matchesWidth, measuredHistogram, normWeight, normWidth, widthPctSpan } from '../mockFilters';
import { createMockApi, mockRows } from '../mock';

const row = (weightClass: number | null, widthClass: number | null = 5, isVariable = false) => ({ weightClass, widthClass, isVariable });

describe('weight/width normalisation matches the backend', () => {
  it('weight: 1–9 → ×100, 0/null → null, outliers clamp to 100..900', () => {
    expect([4, 0, null, 50, 100, 250, 900, 950, 1000].map(normWeight)).toEqual([400, null, null, 100, 100, 250, 900, 900, 900]);
  });
  it('width: only 1..9 are valid', () => {
    expect([0, 1, 5, 9, 10, null].map(normWidth)).toEqual([null, 1, 5, 9, null, null]);
  });
  it('dragging the full 100–900 keeps every font with a usable class (50→100, 950/1000→900), drops 0', () => {
    const rows = [row(0), row(50), row(1000), row(950), row(400)];
    expect(rows.filter(r => matchesWeight(r, [100, 900], false)).length).toBe(4);
  });
});

describe('varAxisMatch', () => {
  it('a Variable font matches when its wght span intersects, only if the toggle is on', () => {
    const vf = row(400, 5, true);
    expect(matchesWeight(vf, [700, 900], false)).toBe(false);
    expect(matchesWeight(vf, [700, 900], true)).toBe(true);
    expect(matchesWeight(vf, [700, 900], true, { wght: [200, 600] })).toBe(false);
    expect(matchesWeight(row(400), [700, 900], true)).toBe(false);
  });
  it('wdth boundaries are half-open: 81.25 belongs to class 4 not 3; class 9 closes at 200', () => {
    expect(widthPctSpan(5, 5)).toEqual([93.75, 106.25]);
    const f = row(400, 5, true);
    expect(matchesWidth(f, [3, 3], true, { wdth: [81.25, 81.25] })).toBe(false);
    expect(matchesWidth(f, [4, 4], true, { wdth: [81.25, 81.25] })).toBe(true);
    expect(matchesWidth(f, [9, 9], true, { wdth: [200, 200] })).toBe(true);
    expect(matchesWidth(row(400, 3), [3, 3], false)).toBe(true);
  });
});

describe('class bins have two series', () => {
  it('default instance once; Variable also in every covered bin', () => {
    const rows = [row(400), row(700), row(400, 5, true)];
    const { bins, spanBins } = classBins(rows, 'weight');
    expect(bins).toEqual([0, 0, 0, 2, 0, 0, 1, 0, 0]);
    expect(spanBins).toEqual([1, 1, 1, 2, 1, 1, 2, 1, 1]);
    expect(classBins(rows, 'width').bins[4]).toBe(3);
  });
});

describe('measured histogram', () => {
  it('v == hi lands in the last bin; below lo in the first', () => {
    expect(binOf(1, 0, 1, 4)).toBe(3);
    expect(binOf(-3, 0, 1, 4)).toBe(0);
    expect(binOf(2, 2, 2, 4)).toBe(0);
  });
  it('all-equal values widen the domain by ±0.5; no data → empty', () => {
    const h = measuredHistogram('stem', 'x_height_ratio', [0.3, 0.3, 0.3], [0.3, 0.3, 0.3], 4);
    expect([h.lo, h.hi, h.count, h.empty]).toEqual([-0.2, 0.8, 3, false]);
    expect(h.bins.reduce((a, b) => a + b, 0)).toBe(3);
    const e = measuredHistogram('stem', 'x_height_ratio', [], [], 4);
    expect(e).toMatchObject({ empty: true, count: 0, lo: 0, hi: 0 });
  });
});

describe('mock api histogram', () => {
  it('has no 1000-row cap, ignores its own filter and keeps the domain fixed while other axes filter', async () => {
    const api = createMockApi();
    const all = await api.axisHistogram({}, 8);
    expect(all).toHaveLength(18);
    expect(all.find(h => h.axis === 'weight_class')!.bins.reduce((a, b) => a + b, 0)).toBe(mockRows().filter(r => normWeight(r.weightClass) != null).length);
    const heavy = await api.axisHistogram({ weightClass: [700, 900], varAxisMatch: { wght: false, wdth: false } }, 8);
    expect(heavy.find(h => h.axis === 'weight_class')!.bins).toEqual(all.find(h => h.axis === 'weight_class')!.bins);
    const stemAll = all.find(h => h.axis === 'stem')!, stemHeavy = heavy.find(h => h.axis === 'stem')!;
    expect([stemHeavy.lo, stemHeavy.hi]).toEqual([stemAll.lo, stemAll.hi]);
    expect(stemHeavy.count).toBeLessThan(stemAll.count);
    const stats = await api.axisStats({ weightClass: [700, 900] });
    expect(stats.find(s => s.axis === 'stem')!.count).toBe(stemAll.count);
  });
});
