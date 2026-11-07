import { describe, expect, it, vi } from 'vitest';
import { createMockApi, mockRows } from '../mock';
import { COMPARE_METRICS, buildCompare } from '../compare';

const keys = (n: number) => mockRows().slice(0, n).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex }));

describe('compareFaces', () => {
  it('returns one column per face, in order, with every metric key present', async () => {
    const api = createMockApi();
    const c = await api.compareFaces(keys(3));
    expect(c.faces.map(f => f.face)).toEqual(keys(3));
    for (const f of c.faces) expect(Object.keys(f.metrics).sort()).toEqual([...COMPARE_METRICS].sort());
  });

  it('keeps quality, reason and percentile from the reading; unmeasured metrics become failed, not zero', async () => {
    const api = createMockApi();
    const d = await api.getFace(keys(1)[0], 'vi');
    const patched = { ...d, axes: d.axes.filter(a => a.key !== 'contrast_h').map(a => a.key === 'stress_deg' ? { ...a, quality: 'estimated', percentile: 71 } : a) };
    const c = buildCompare([patched]);
    expect(c.faces[0].metrics.stress_deg).toMatchObject({ quality: 'estimated', percentile: 71 });
    expect(c.faces[0].metrics.contrast_h).toMatchObject({ value: null, quality: 'failed' });
  });

  it('carries Vietnamese overflow, classification, license and designer', async () => {
    const api = createMockApi();
    const c = await api.compareFaces(keys(2));
    const f = c.faces[0];
    expect(f.vi).toMatchObject({ supports: expect.any(Boolean), stackedMarksOverflow: expect.any(Boolean) });
    expect(f.classification.class).toBeTruthy();
    expect('license' in f).toBe(true);
  });

  it('rejects fewer than two faces and more than eight', async () => {
    const api = createMockApi();
    await expect(api.compareFaces(keys(1))).rejects.toThrow();
    await expect(api.compareFaces(keys(9))).rejects.toThrow();
  });

  it('fails the whole call if one face cannot be read (no partial columns)', async () => {
    const api = createMockApi();
    vi.spyOn(api, 'getFace').mockRejectedValueOnce(new Error('gone'));
    await expect(api.compareFaces(keys(2))).rejects.toThrow('gone');
  });
});
