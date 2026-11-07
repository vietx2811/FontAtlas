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

  it('one unreadable face is reported with its reason; the rest still compare', async () => {
    const api = createMockApi();
    vi.spyOn(api, 'getFace').mockRejectedValueOnce(new Error('gone'));
    const c = await api.compareFaces(keys(3));
    expect(c.faces).toHaveLength(2);
    expect(c.failed).toEqual([{ face: keys(3)[0], reason: 'gone' }]);
  });

  it('fails when fewer than two faces can be read', async () => {
    const api = createMockApi();
    vi.spyOn(api, 'getFace').mockRejectedValue(new Error('gone'));
    await expect(api.compareFaces(keys(2))).rejects.toThrow('gone');
  });

  it('tolerates sparse real data: missing axes, vietnamese, metadata and usage', async () => {
    const api = createMockApi();
    const d = await api.getFace(keys(1)[0], 'vi');
    const sparse = { ...d, axes: undefined, vietnamese: undefined, metadata: undefined, classificationInfo: { ...d.classificationInfo, usage: undefined, signals: undefined } } as never;
    const c = buildCompare([sparse]);
    expect(c.faces[0].metrics.x_height_em.quality).toBe('failed');
    expect(c.faces[0].vi).toMatchObject({ supports: false, missing: 0, stackedMarksOverflow: false });
    expect(c.faces[0].license).toBeNull();
    expect(c.faces[0].classification.usage).toEqual([]);
  });
});
