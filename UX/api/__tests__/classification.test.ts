import { describe, expect, it } from 'vitest';
import { createMockApi } from '../mock';

const key = (r: { hashHex: string; faceIndex: number }) => ({ hashHex: r.hashHex, faceIndex: r.faceIndex });

describe('mock classification', () => {
  it('classFacets counts per class and ignores the class filter itself', async () => {
    const api = createMockApi();
    const all = await api.queryFaces({});
    const facets = await api.classFacets({});
    expect(facets.reduce((n, f) => n + f.count, 0)).toBe(all.total);
    const serif = facets.find(f => f.class === 'serif')!;
    const filtered = await api.classFacets({ classes: ['sans'] });
    expect(filtered.find(f => f.class === 'serif')!.count).toBe(serif.count);
    expect((await api.queryFaces({ classes: ['serif'] })).total).toBe(serif.count);
    expect(facets.every(f => f.class !== 'display')).toBe(true);
  });

  it('subclass filter ANDs with class and uses snake_case', async () => {
    const api = createMockApi();
    const serif = (await api.classFacets({})).find(f => f.class === 'serif')!;
    const sub = serif.subclasses[0];
    expect(sub.subclass).toMatch(/^[a-z_]+$/);
    const r = await api.queryFaces({ classes: ['serif'], subclasses: [sub.subclass] });
    expect(r.total).toBe(sub.count);
  });

  it('override wins in detail, facets and filter; clearing restores the auto result', async () => {
    const api = createMockApi();
    const face = key((await api.queryFaces({ classes: ['sans'] })).items[0]);
    const before = (await api.getFace(face, 'vi')).classificationInfo;
    expect(before.source).toBe('auto');
    await api.setClassificationOverride({ face, class: 'script', subclass: null });
    const after = (await api.getFace(face, 'vi')).classificationInfo;
    expect(after).toMatchObject({ class: 'script', source: 'override', autoClass: 'sans' });
    expect((await api.queryFaces({ classes: ['script'] })).items.map(key)).toContainEqual(face);
    await api.clearClassificationOverride(face);
    expect((await api.getFace(face, 'vi')).classificationInfo).toMatchObject({ class: 'sans', source: 'auto', autoClass: null });
  });

  it('low-confidence fonts are uncertain with a candidate', async () => {
    const api = createMockApi();
    const u = (await api.queryFaces({ classes: ['uncertain'] })).items[0];
    const info = (await api.getFace(key(u), 'vi')).classificationInfo;
    expect(info).toMatchObject({ class: 'uncertain', candidate: expect.any(String) });
  });
});
