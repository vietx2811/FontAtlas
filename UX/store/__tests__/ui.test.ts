import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../api/mock';
import { createUiStore, toGridQuery, faceKey, PERSIST_DEBOUNCE_MS, RECENT_MAX, RECENT_DEBOUNCE_MS } from '../ui';

describe('toGridQuery', () => {
  const base = { view: 'all', query: '', viOnly: false, recentViews: [], active: new Map<string, readonly number[]>() } as const;
  it('maps every sidebar view to query_faces params', () => {
    expect(toGridQuery({ ...base })).toEqual({});
    expect(toGridQuery({ ...base, view: 'star' })).toEqual({ favoritesOnly: true });
    expect(toGridQuery({ ...base, view: 'col:7' })).toEqual({ collectionId: 7 });
    expect(toGridQuery({ ...base, view: 'src:3' })).toEqual({ sourceIds: [3] });
  });
  it('adds tag (from filters.tagIds), text and vi-only', () => {
    expect(toGridQuery({ ...base, filters: { tagIds: [4] }, query: '  phở ', viOnly: true })).toEqual({ tagIds: [4], text: 'phở', vi: 'full' });
  });
  it('recent = faceKeys in viewing order (newest first), kept in that order by sort=match', () => {
    const recentViews = [{ face: 'bb:0', at: 2 }, { face: 'aa:1', at: 1 }];
    expect(toGridQuery({ ...base, view: 'recent', recentViews })).toEqual({
      faceKeys: [{ hashHex: 'bb', faceIndex: 0 }, { hashHex: 'aa', faceIndex: 1 }], sort: 'match',
    });
  });
  it('active = faceKeys built from the `faces` of each activated file (TTC: every face), combinable with the vi-only and text filters', () => {
    expect(toGridQuery({ ...base, view: 'active', active: new Map([['h1', [0]], ['h2', [0, 1]]]), viOnly: true, query: 'x' })).toEqual({
      faceKeys: [{ hashHex: 'h1', faceIndex: 0 }, { hashHex: 'h2', faceIndex: 0 }, { hashHex: 'h2', faceIndex: 1 }], sort: 'match', text: 'x', vi: 'full',
    });
  });
  it('an empty recent/active list is an empty faceKeys list, never "no filter"', () => {
    expect(toGridQuery({ ...base, view: 'recent' })).toEqual({ faceKeys: [], sort: 'match' });
    expect(toGridQuery({ ...base, view: 'active' })).toEqual({ faceKeys: [], sort: 'match' });
  });
  it('caps faceKeys at 2000 (6a limit)', () => {
    const active = new Map(Array.from({ length: 2500 }, (_, i) => [`h${i}`, [0]] as [string, number[]]));
    expect(toGridQuery({ ...base, view: 'active', active })!.faceKeys).toHaveLength(2000);
  });
});

describe('ui store', () => {
  it('tagIds live in filters: the sidebar tag replaces them, a view change keeps them, bấm lại bỏ; clears the selection style', () => {
    const s = createUiStore();
    s.getState().setTag('2');
    expect(s.getState().filters.tagIds).toEqual([2]);
    s.getState().setTag('3');
    expect(s.getState().filters.tagIds).toEqual([3]);
    s.getState().setView('star');
    expect(s.getState().filters.tagIds).toEqual([3]); // đổi view không xoá thẻ đã chọn
    s.getState().setTag(null);
    expect(s.getState().filters.tagIds).toBeUndefined();
    s.getState().select('aa:0');
    s.getState().setStyle('Bold');
    s.getState().select('bb:0');
    expect(s.getState().style).toBeNull();
  });
  it('clamps sample size to 18..72', () => {
    const s = createUiStore();
    s.getState().setSampleSize(5);
    expect(s.getState().sampleSize).toBe(18);
    s.getState().setSampleSize(500);
    expect(s.getState().sampleSize).toBe(72);
  });
  it('select records recent views: newest first, no duplicates, max 200', () => {
    const s = createUiStore();
    s.getState().select('a:0'); s.getState().select('b:0'); s.getState().select('a:0');
    expect(s.getState().recentViews.map(r => r.face)).toEqual(['a:0', 'b:0']);
    for (let i = 0; i < RECENT_MAX + 20; i++) s.getState().select(`h${i}:0`);
    expect(s.getState().recentViews).toHaveLength(RECENT_MAX);
    expect(s.getState().recentViews[0].face).toBe(`h${RECENT_MAX + 19}:0`);
    s.getState().select(null);
    expect(s.getState().recentViews).toHaveLength(RECENT_MAX);
  });
  it('faceKey is hash:index', () => {
    expect(faceKey({ hashHex: 'ab', faceIndex: 2 })).toBe('ab:2');
  });
});

describe('persistence', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('hydrates from get_ui_state', async () => {
    const api = createMockApi();
    await api.setUiState({ sidebarCollapsed: true, openSections: ['col'], closedSections: [], layout: 'list', sampleSize: 50, infoExpanded: true });
    const s = createUiStore();
    await s.getState().hydrate(api);
    const st = s.getState();
    expect([st.sidebarCollapsed, st.layout, st.sampleSize, st.infoExpanded]).toEqual([true, 'list', 50, true]);
    expect(st.openSections).toEqual({ col: true, src: false, tag: false, case: true });
  });

  it('debounces writes through set_ui_state and ignores non-persisted fields', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'setUiState');
    const s = createUiStore();
    await s.getState().hydrate(api);
    s.getState().setQuery('abc'); // không bền
    s.getState().setSampleSize(40);
    s.getState().setSampleSize(41);
    s.getState().toggleSidebar();
    expect(spy).not.toHaveBeenCalled();
    vi.advanceTimersByTime(PERSIST_DEBOUNCE_MS + 1);
    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy.mock.calls[0][0]).toMatchObject({ sampleSize: 41, sidebarCollapsed: true, layout: 'grid' });
    vi.advanceTimersByTime(5000);
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it('does not write back the state it just hydrated', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'setUiState');
    const s = createUiStore();
    await s.getState().hydrate(api);
    vi.advanceTimersByTime(5000);
    expect(spy).not.toHaveBeenCalled();
  });
});

describe('recent views persistence (ui.recent_views)', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('hydrates from get_setting and ignores corrupt JSON', async () => {
    const api = createMockApi();
    await api.setSetting('ui.recent_views', JSON.stringify([{ face: 'a:0', at: 5 }]));
    const s = createUiStore();
    await s.getState().hydrate(api);
    expect(s.getState().recentViews).toEqual([{ face: 'a:0', at: 5 }]);
    await api.setSetting('ui.recent_views', '{not json');
    const t = createUiStore();
    await t.getState().hydrate(api);
    expect(t.getState().recentViews).toEqual([]);
  });

  it('writes the list 1 s after the last view, merged into a single set_setting', async () => {
    expect(RECENT_DEBOUNCE_MS).toBe(1000);
    const api = createMockApi();
    const spy = vi.spyOn(api, 'setSetting');
    const s = createUiStore();
    await s.getState().hydrate(api);
    s.getState().select('a:0'); vi.advanceTimersByTime(600); s.getState().select('b:0');
    vi.advanceTimersByTime(999);
    expect(spy).not.toHaveBeenCalled();
    vi.advanceTimersByTime(2);
    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy.mock.calls[0][0]).toBe('ui.recent_views');
    expect(JSON.parse(spy.mock.calls[0][1]).map((r: { face: string }) => r.face)).toEqual(['b:0', 'a:0']);
  });
});

describe('nhóm Case: mở/đóng được lưu (closedSections)', () => {
  it('dữ liệu cũ không có closedSections → Case mở; đóng rồi lưu closedSections=[case]; mở lại thì rỗng', async () => {
    const api = createMockApi();
    const old = { sidebarCollapsed: false, openSections: ['col', 'src', 'tag'], layout: 'grid', sampleSize: 34, infoExpanded: false }; // lưu trước khi có nhóm Case
    await api.setUiState(old as never);
    const s = createUiStore(); await s.getState().hydrate(api);
    expect(s.getState().openSections.case).toBe(true);
    vi.useFakeTimers();
    s.getState().toggleSection('case');
    await vi.advanceTimersByTimeAsync(PERSIST_DEBOUNCE_MS + 50);
    expect((await api.getUiState()).closedSections).toEqual(['case']);
    s.getState().toggleSection('case');
    await vi.advanceTimersByTimeAsync(PERSIST_DEBOUNCE_MS + 50);
    expect((await api.getUiState()).closedSections).toEqual([]);
    vi.useRealTimers();
  });
});
