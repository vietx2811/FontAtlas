import { describe, expect, it } from 'vitest';
import combos from '../../bindings/fixtures/pair_these_combos.json';
import { createMockApi, mockRows } from '../mock';

const keys = (n: number) => mockRows().slice(0, n).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex }));

describe('mock pairThese', () => {
  it('returns 1–3 ranked combos, heading and body from the chosen faces, no score field', async () => {
    const api = createMockApi();
    const r = await api.pairThese(keys(3), { maxCombos: 3 });
    expect(r.length).toBeGreaterThanOrEqual(1); expect(r.length).toBeLessThanOrEqual(3);
    expect(r.map(c => c.rank)).toEqual(r.map((_, i) => i + 1));
    const set = new Set(keys(3).map(k => `${k.hashHex}:${k.faceIndex}`));
    for (const c of r) {
      expect(set.has(`${c.heading.hashHex}:${c.heading.faceIndex}`)).toBe(true);
      expect(set.has(`${c.body.hashHex}:${c.body.faceIndex}`)).toBe(true);
      expect(c.heading.faceId).not.toBe(c.body.faceId);
      expect(JSON.stringify(c)).not.toMatch(/score/i);
    }
  });
  it('maxCombos is honoured and the role assignments differ between combos', async () => {
    const api = createMockApi();
    expect(await api.pairThese(keys(2), { maxCombos: 1 })).toHaveLength(1);
    const two = await api.pairThese(keys(2), { maxCombos: 3 });
    const pairs = new Set(two.map(c => `${c.heading.faceId}>${c.body.faceId}`));
    expect(pairs.size).toBe(two.length);
  });
  it('rejects fewer than two or more than four faces', async () => {
    const api = createMockApi();
    await expect(api.pairThese(keys(1))).rejects.toThrow();
    await expect(api.pairThese(keys(5))).rejects.toThrow();
  });
  it('the backend fixture has the shape the UI needs', () => {
    const c = combos[0] as unknown as { heading: object; body: object; rank: number; assessment: { labels: Array<{ symbol: string }>; summary: string } };
    expect(c.rank).toBe(1); expect(c.assessment.labels[0].symbol).toBeTruthy(); expect(c.assessment.summary).toBeTruthy();
  });
});
