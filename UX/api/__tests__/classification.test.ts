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
    // v2: Slab, Handwriting và Display có trong dữ liệu mock.
    for (const c of ['slab', 'handwriting', 'display']) expect(facets.some(f => f.class === c), c).toBe(true);
  });

  it('includeSecondary: a hybrid face is ALSO counted under its secondary class, so facet totals can exceed the number of faces; filters match it too', async () => {
    const api = createMockApi();
    const all = await api.queryFaces({});
    const plain = await api.classFacets({});
    const withSec = await api.classFacets({ includeSecondary: true });
    const sum = (f: typeof plain) => f.reduce((n, x) => n + x.count, 0);
    expect(sum(plain)).toBe(all.total);
    expect(sum(withSec)).toBeGreaterThan(all.total);
    expect(withSec.find(f => f.class === 'sans')!.count).toBe(plain.find(f => f.class === 'sans')!.count + 1);
    expect((await api.queryFaces({ classes: ['sans'], includeSecondary: true })).total).toBe(plain.find(f => f.class === 'sans')!.count + 1);
    expect((await api.queryFaces({ classes: ['sans'] })).total).toBe(plain.find(f => f.class === 'sans')!.count);
  });

  it('an override sent with v1 labels is STORED as v2 and drops the secondary class', async () => {
    const api = createMockApi();
    const face = { hashHex: (await api.queryFaces({ text: 'Caveat' })).items[0].hashHex, faceIndex: 0 };
    await api.setClassificationOverride({ face, class: 'script', subclass: 'formal' });
    const d = await api.getFace(face, 'vi');
    expect(d.classificationInfo).toMatchObject({ class: 'script', subclass: 'formal_copperplate', source: 'override', secondary: null });
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
