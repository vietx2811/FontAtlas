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
    expect(err.detail).toEqual({ existingId: s1.id, existingPath: s1.path });
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

  it('faceKeys restricts to those faces, in the given order with sort=match (Gần đây / Đang bật)', async () => {
    const api = createMockApi();
    const all = (await api.queryFaces({})).items;
    const keys = [all[5], all[1], all[9]].map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex }));
    const page = await api.queryFaces({ faceKeys: keys, sort: 'match' });
    expect(page.items.map(r => r.faceId)).toEqual([all[5].faceId, all[1].faceId, all[9].faceId]);
    expect(page.total).toBe(3);
    expect((await api.queryFaces({ faceKeys: [] })).total).toBe(0);
    const err = await api.queryFaces({ faceKeys: Array.from({ length: 2001 }, () => keys[0]) }).catch(e => e);
    expect(err.code).toBe('invalid_arg');
  });

  it('get/set_setting accept only ui.* keys up to 64 KB', async () => {
    const api = createMockApi();
    expect(await api.getSetting('ui.recent_views')).toBeNull();
    await api.setSetting('ui.recent_views', '[]');
    expect(await api.getSetting('ui.recent_views')).toBe('[]');
    expect((await api.setSetting('embed_enabled', '1').catch(e => e)).code).toBe('invalid_arg');
    expect((await api.setSetting('ui.' + 'x'.repeat(61), '1').catch(e => e)).code).toBe('invalid_arg');
    expect((await api.setSetting('ui.big', 'x'.repeat(65_537)).catch(e => e)).code).toBe('invalid_arg');
    await api.setSetting('ui.max', 'x'.repeat(65_536));
  });

  it('previews use visibly different fonts per family (system fonts, so the dev grid is not one fallback)', async () => {
    const api = createMockApi();
    const items = (await api.queryFaces({})).items;
    const fam = (i: number) => decodeURIComponent(api.previewUrl(items[i], { text: 'x', px: 48, generation: 0, rendererVersion: 'm' })).match(/font-family="([^"]+)"/)![1];
    const distinct = new Set(items.map((_, i) => fam(i)));
    expect(distinct.size).toBeGreaterThanOrEqual(8);
  });
});

describe('mock analysis data (tabs of the detail panel)', () => {
  it('get_face carries 21 axis readings with a mix of measured / estimated / failed and a percentile only when measured or estimated', async () => {
    const api = createMockApi();
    const r = (await api.queryFaces({ text: 'Inter', limit: 1 })).items[0];
    const d = await api.getFace({ hashHex: r.hashHex, faceIndex: 0 }, 'vi');
    expect(d.axes).toHaveLength(21);
    const q = new Set(d.axes.map(a => a.quality));
    expect(q.has('measured')).toBe(true);
    for (const a of d.axes) {
      if (a.quality === 'failed') { expect(a.percentile).toBeNull(); expect(a.reason).toBeTruthy(); }
      else if (['terminal', 'a_storeys', 'g_storeys', 'serif_foot', 'cap_height_em', 'descender_em'].includes(a.key)) expect(a.percentile).toBeNull(); // phân loại, không có phân vị
      else expect(a.percentile).toBeGreaterThanOrEqual(0);
    }
    expect(d.axes.map(a => a.key)).toEqual(expect.arrayContaining(['x_height_em', 'cap_height_em', 'terminal', 'a_storeys', 'g_storeys']));
  });

  it('is deterministic per face and the Vietnamese block follows the row (missing list length = viMissing)', async () => {
    const api = createMockApi();
    const rows = (await api.queryFaces({})).items;
    const bad = rows.find(r => r.viMissing > 0)!;
    const a = await api.getFace({ hashHex: bad.hashHex, faceIndex: 0 }, 'vi');
    const b = await api.getFace({ hashHex: bad.hashHex, faceIndex: 0 }, 'vi');
    expect(a.axes).toEqual(b.axes);
    expect(a.vietnamese.missing).toHaveLength(bad.viMissing);
    expect(a.vietnamese.total).toBeGreaterThan(100);
  });

  it('similar: 3 hits in similarTop, never the face itself, with same/differ and a source', async () => {
    const api = createMockApi();
    const r = (await api.queryFaces({ text: 'Roboto', limit: 1 })).items[0];
    const d = await api.getFace({ hashHex: r.hashHex, faceIndex: 0 }, 'vi');
    expect(d.similarTop).toHaveLength(3);
    const hits = await api.similar({ hashHex: r.hashHex, faceIndex: 0 }, { k: 5 });
    expect(hits).toHaveLength(5);
    expect(hits.every(h => h.row.faceId !== r.faceId && h.same.length > 0 && h.differ.length > 0)).toBe(true);
    expect(new Set(hits.map(h => h.source)).size).toBeGreaterThanOrEqual(1);
  });

  it('pair_candidates: mode changes which relations appear; safe never contains ◆; empty when the heading lacks Vietnamese and project_vi is on', async () => {
    const api = createMockApi();
    const good = (await api.queryFaces({ text: 'Playfair', limit: 1 })).items[0];
    const base = { role: 'heading' as const, projectVi: true, k: 8, collectionId: null, lang: 'vi' as const };
    const safe = await api.pairCandidates({ hashHex: good.hashHex, faceIndex: 0 }, { ...base, mode: 'safe' });
    expect(safe.length).toBeGreaterThan(0);
    expect(safe.every(c => c.labels.every(l => l.symbol !== '◆'))).toBe(true);
    expect(safe.every(c => !c.row.family.includes('Playfair'))).toBe(true);
    const exp = await api.pairCandidates({ hashHex: good.hashHex, faceIndex: 0 }, { ...base, mode: 'experimental' });
    expect(exp.some(c => c.labels.some(l => l.symbol === '◆'))).toBe(true);
    const bad = (await api.queryFaces({})).items.find(r => r.viMissing > 0)!;
    expect(await api.pairCandidates({ hashHex: bad.hashHex, faceIndex: 0 }, { ...base, mode: 'safe' })).toEqual([]);
    expect((await api.pairCandidates({ hashHex: bad.hashHex, faceIndex: 0 }, { ...base, mode: 'safe', projectVi: false })).length).toBeGreaterThan(0);
  });

  it('one source carries a last scan with files still in the cloud (for QA and screenshots)', async () => {
    const api = createMockApi();
    const src = (await api.listSources()).find(x => x.id === 2)!;
    expect(src.lastScan?.notDownloaded).toBe(12);
    expect((await api.listSources()).filter(x => (x.lastScan?.notDownloaded ?? 0) > 0)).toHaveLength(1);
  });
});
