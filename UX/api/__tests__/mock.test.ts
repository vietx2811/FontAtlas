import { describe, expect, it } from 'vitest';
import { CmdError } from '../types';
import { createMockApi, mockRows } from '../mock';

const key = (r: { hashHex: string; faceIndex: number }) => ({ hashHex: r.hashHex, faceIndex: r.faceIndex });

describe('mock api', () => {
  it('filters by source, favorites, collection, tag, vi and text', async () => {
    const api = createMockApi();
    const all = await api.queryFaces({});
    expect(all.total).toBe(mockRows().length);

    const [s1] = await api.listSources();
    const bySrc = await api.queryFaces({ sourceIds: [s1.id] });
    expect(bySrc.total).toBe(s1.faceCount);

    const first = all.items[0];
    await api.setFavorite(key(first), true);
    expect((await api.queryFaces({ favoritesOnly: true })).items.map(r => r.faceId)).toEqual([first.faceId]);

    const col = await api.createCollection('Menu', null);
    await api.addToCollection(col.id, [key(first)]);
    expect((await api.queryFaces({ collectionId: col.id })).total).toBe(1);
    expect((await api.listCollections()).find(c => c.id === col.id)?.itemCount).toBe(1);

    const tag = await api.createTag('serif');
    await api.tagFaces(tag.id, [key(first)]);
    expect((await api.queryFaces({ tagIds: [tag.id] })).total).toBe(1);
    expect((await api.tagsOf(key(first))).map(t => t.name)).toEqual(['serif']);

    const vi = await api.queryFaces({ vi: 'full' });
    expect(vi.items.every(r => r.viMissing === 0)).toBe(true);
    expect(vi.total).toBeLessThan(all.total);

    expect((await api.queryFaces({ text: 'ROBOTO' })).items.every(r => /roboto/i.test(r.family))).toBe(true);
  });

  it('rejects a nested source with the existing id', async () => {
    const api = createMockApi();
    const [s1] = await api.listSources();
    const err = await api.addSource(`${s1.path}/sub`).catch(e => e);
    expect(err).toBeInstanceOf(CmdError);
    expect(err.code).toBe('nested_source');
    expect(err.detail).toEqual({ existingId: s1.id });
  });

  it('adds a source and lists it; removes it again', async () => {
    const api = createMockApi();
    const n = (await api.listSources()).length;
    const s = await api.addSource('/Users/me/NewFonts');
    expect((await api.listSources()).length).toBe(n + 1);
    await api.removeSource(s.id);
    expect((await api.listSources()).length).toBe(n);
  });

  it('family styles: statics sorted input, same family only', async () => {
    const api = createMockApi();
    const styles = await api.getFamilyStyles('Public Sans');
    expect(styles.length).toBeGreaterThan(3);
    expect(styles.some(s => s.italic)).toBe(true);
    expect((await api.getFamilyStyles('Nope')).length).toBe(0);
  });

  it('persists ui state and activation toggles', async () => {
    const api = createMockApi();
    await api.setUiState({ sidebarCollapsed: true, openSections: ['col'], layout: 'list', sampleSize: 50, infoExpanded: true });
    expect((await api.getUiState()).layout).toBe('list');
    const r = (await api.queryFaces({})).items[0];
    await api.activationActivate([key(r)]);
    expect((await api.activationState()).find(s => s.hashHex === r.hashHex)?.state).toBe('active');
    await api.activationDeactivate([key(r)]);
    expect((await api.activationState()).find(s => s.hashHex === r.hashHex)?.state ?? 'inactive').toBe('inactive');
  });

  it('getFace returns detail for a known face and not_found otherwise', async () => {
    const api = createMockApi();
    const r = (await api.queryFaces({})).items[0];
    const d = await api.getFace(key(r), 'vi');
    expect(d.row.faceId).toBe(r.faceId);
    expect(d.file.numGlyphs).toBeGreaterThan(0);
    const err = await api.getFace({ hashHex: 'ff', faceIndex: 0 }, 'vi').catch(e => e);
    expect(err.code).toBe('not_found');
  });
});
