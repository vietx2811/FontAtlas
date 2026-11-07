import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Api } from '../../../api';
import { makeRow } from '../../../features/library-grid/api';
import type { FaceRow, GridQuery } from '../../../features/library-grid/types';
import { useNoticeStore } from '../../../store/notices';
import { SELECT_MAX, rowKey, useSelectionStore } from '../../../store/selection';
import { faceKey, useUiStore } from '../../../store/ui';
import { useLibrarySelection } from '../useLibrarySelection';

const initial = useUiStore.getState();
const flush = () => act(async () => { for (let i = 0; i < 12; i++) await new Promise(r => setTimeout(r, 0)); });
const keysOf = () => [...useSelectionStore.getState().rows.keys()];

/** Truy vấn giả: thứ tự do `order()` quyết định, đổi được giữa các lần gọi. */
function fakeApi(order: () => FaceRow[], hook?: (q: { offset?: number; limit?: number }) => Promise<void>) {
  const api = {
    queryFaces: vi.fn(async (q: { offset?: number; limit?: number }) => {
      await hook?.(q);
      const all = order(); const off = q.offset ?? 0;
      return { total: all.length, offset: off, items: all.slice(off, off + (q.limit ?? 100)) };
    }),
  };
  return api as unknown as Api & typeof api;
}
function setup(api: Api, query: GridQuery = {}) {
  const container = { current: null as HTMLElement | null };
  return renderHook(({ q }: { q: GridQuery }) => useLibrarySelection(api, q, 'scope', container), { initialProps: { q: query } });
}

describe('useLibrarySelection: ranges', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useSelectionStore.getState().clear(); useNoticeStore.setState({ notices: [] }); });

  it('a range over the cap keeps the part next to the clicked card (down), selects the clicked card and never clears', async () => {
    const rows = Array.from({ length: 5000 }, (_, i) => makeRow(i));
    const api = fakeApi(() => rows);
    const { result } = setup(api);
    act(() => result.current.pick(rows[0], 0, { meta: false, shift: false }));
    act(() => result.current.pick(rows[4000], 4000, { meta: false, shift: true }));
    await flush();
    const keys = keysOf();
    expect(keys).toHaveLength(SELECT_MAX);
    expect(keys).toContain(rowKey(rows[4000]));
    expect(keys).toContain(rowKey(rows[4000 - SELECT_MAX + 1]));
    expect(keys).not.toContain(rowKey(rows[0]));
    expect(useUiStore.getState().selectedId).toBe(rowKey(rows[4000]));
    expect(useNoticeStore.getState().notices.map(n => n.text).join()).toMatch(/2\.000 font mỗi lần/);
  });

  it('…and upwards: keeps the part next to the clicked card above the anchor', async () => {
    const rows = Array.from({ length: 5000 }, (_, i) => makeRow(i));
    const { result } = setup(fakeApi(() => rows));
    act(() => result.current.pick(rows[4000], 4000, { meta: false, shift: false }));
    act(() => result.current.pick(rows[100], 100, { meta: false, shift: true }));
    await flush();
    const keys = keysOf();
    expect(keys).toHaveLength(SELECT_MAX);
    expect(keys).toContain(rowKey(rows[100]));
    expect(keys).toContain(rowKey(rows[100 + SELECT_MAX - 1]));
    expect(keys).not.toContain(rowKey(rows[4000]));
    expect(useUiStore.getState().selectedId).toBe(rowKey(rows[100]));
  });

  it('the anchor is a key: when the order changes, the range is taken from where the anchor is NOW', async () => {
    let rows = Array.from({ length: 12 }, (_, i) => makeRow(i));
    const api = fakeApi(() => rows);
    const { result } = setup(api);
    const anchor = rows[2];
    act(() => result.current.pick(anchor, 2, { meta: false, shift: false }));
    rows = [...rows.filter(r => r !== anchor)]; rows.splice(5, 0, anchor); // mốc nhảy từ index 2 sang 5
    act(() => result.current.pick(rows[8], 8, { meta: false, shift: true }));
    await flush();
    expect(keysOf().sort()).toEqual(rows.slice(5, 9).map(rowKey).sort());
    expect(keysOf()).toContain(rowKey(anchor));
  });

  it('anchor no longer in the query → behaves like a plain click', async () => {
    let rows = Array.from({ length: 12 }, (_, i) => makeRow(i));
    const { result } = setup(fakeApi(() => rows));
    const anchor = rows[2];
    act(() => result.current.pick(anchor, 2, { meta: false, shift: false }));
    rows = rows.filter(r => r !== anchor);
    act(() => result.current.pick(rows[6], 6, { meta: false, shift: true }));
    await flush();
    expect(keysOf()).toEqual([rowKey(rows[6])]);
    expect(useUiStore.getState().selectedId).toBe(rowKey(rows[6]));
  });

  it('the anchor is dropped when the query changes (membership of Gần đây/Đang bật included)', async () => {
    const rows = Array.from({ length: 12 }, (_, i) => makeRow(i));
    const { result, rerender } = setup(fakeApi(() => rows), {});
    act(() => result.current.pick(rows[2], 2, { meta: false, shift: false }));
    expect(useSelectionStore.getState().anchor).toEqual({ key: rowKey(rows[2]), index: 2 });
    rerender({ q: { faceKeys: [{ hashHex: rows[1].hashHex, faceIndex: 0 }] } });
    expect(useSelectionStore.getState().anchor).toBeNull();
    act(() => result.current.pick(rows[6], 6, { meta: false, shift: true }));
    await flush();
    expect(keysOf()).toEqual([rowKey(rows[6])]);
  });

  it('a failure of an older Shift request is ignored when a newer click has taken over', async () => {
    const rows = Array.from({ length: 12 }, (_, i) => makeRow(i));
    let fail = false; let release!: () => void;
    const gate = new Promise<void>(r => (release = r));
    const api = fakeApi(() => rows, async () => { if (fail) { await gate; throw new Error('mạng chập chờn'); } });
    const { result } = setup(api);
    act(() => result.current.pick(rows[1], 1, { meta: false, shift: false }));
    fail = true;
    act(() => result.current.pick(rows[5], 5, { meta: false, shift: true })); // sẽ lỗi, chậm
    fail = false;
    act(() => result.current.pick(rows[9], 9, { meta: false, shift: false })); // click mới thay thế
    release(); await flush();
    expect(useNoticeStore.getState().notices).toHaveLength(0);
    expect(keysOf()).toEqual([rowKey(rows[9])]);
  });

  it('a failure of the CURRENT Shift request is reported', async () => {
    const rows = Array.from({ length: 12 }, (_, i) => makeRow(i));
    let fail = false;
    const api = fakeApi(() => rows, async () => { if (fail) throw new Error('mất kết nối'); });
    const { result } = setup(api);
    act(() => result.current.pick(rows[1], 1, { meta: false, shift: false }));
    fail = true;
    act(() => result.current.pick(rows[5], 5, { meta: false, shift: true }));
    await flush();
    expect(useNoticeStore.getState().notices.map(n => n.text).join()).toMatch(/mất kết nối/);
    expect(faceKey(rows[1])).toBeTruthy();
  });
});
