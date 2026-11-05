import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../api/mock';
import { createUiStore, toGridQuery, faceKey, PERSIST_DEBOUNCE_MS } from '../ui';

describe('toGridQuery', () => {
  const base = { view: 'all', tag: null, query: '', viOnly: false } as const;
  it('maps every sidebar view to query_faces params', () => {
    expect(toGridQuery({ ...base })).toEqual({});
    expect(toGridQuery({ ...base, view: 'star' })).toEqual({ favoritesOnly: true });
    expect(toGridQuery({ ...base, view: 'col:7' })).toEqual({ collectionId: 7 });
    expect(toGridQuery({ ...base, view: 'src:3' })).toEqual({ sourceIds: [3] });
  });
  it('adds tag, text and vi-only', () => {
    expect(toGridQuery({ ...base, tag: '4', query: '  phở ', viOnly: true })).toEqual({ tagIds: [4], text: 'phở', vi: 'full' });
  });
  it('returns null for views the backend cannot filter yet (recent, active)', () => {
    expect(toGridQuery({ ...base, view: 'recent' })).toBeNull();
    expect(toGridQuery({ ...base, view: 'active' })).toBeNull();
  });
});

describe('ui store', () => {
  it('resets tag when switching view and clears the selection style', () => {
    const s = createUiStore();
    s.getState().setTag('2');
    s.getState().setView('star');
    expect(s.getState().tag).toBeNull();
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
  it('faceKey is hash:index', () => {
    expect(faceKey({ hashHex: 'ab', faceIndex: 2 })).toBe('ab:2');
  });
});

describe('persistence', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('hydrates from get_ui_state', async () => {
    const api = createMockApi();
    await api.setUiState({ sidebarCollapsed: true, openSections: ['col'], layout: 'list', sampleSize: 50, infoExpanded: true });
    const s = createUiStore();
    await s.getState().hydrate(api);
    const st = s.getState();
    expect([st.sidebarCollapsed, st.layout, st.sampleSize, st.infoExpanded]).toEqual([true, 'list', 50, true]);
    expect(st.openSections).toEqual({ col: true, src: false, tag: false });
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
