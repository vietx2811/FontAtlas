import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockLibraryApi, makeRow } from '../api';
import { PAGE_SIZE, PagedFaces, SEARCH_DEBOUNCE_MS } from '../pagedFaces';
import type { FacePage, FaceQuery } from '../types';

const rows = Array.from({ length: 1000 }, (_, i) => makeRow(i));
/** Chạy hết microtask (mock trả Promise đã resolve; không dùng setTimeout vì đồng hồ giả). */
const flush = async () => { for (let i = 0; i < 10; i++) await Promise.resolve(); };
function deferredApi() {
  const calls: FaceQuery[] = []; const resolvers: Array<(p: FacePage) => void> = [];
  return { calls, resolvers, queryFaces: (q: FaceQuery) => { calls.push(q); return new Promise<FacePage>(r => resolvers.push(r)); } };
}

beforeEach(() => vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] }));
afterEach(() => vi.useRealTimers());

describe('PagedFaces', () => {
  it('fetches page 0 immediately and learns total', async () => {
    const api = createMockLibraryApi(rows);
    const p = new PagedFaces(api, {});
    await flush();
    expect(api.calls).toEqual([{ offset: 0, limit: PAGE_SIZE }]);
    expect(p.getSnapshot().total).toBe(1000);
    expect(p.rowAt(0)?.faceId).toBe(1);
    expect(p.rowAt(PAGE_SIZE)).toBeUndefined();
  });

  it('only fetches missing pages around the visible range', async () => {
    const api = createMockLibraryApi(rows);
    const p = new PagedFaces(api, {});
    await flush();
    p.ensureRange(0, 40); p.ensureRange(0, 40);
    p.ensureRange(150, 190);
    await flush();
    expect(api.calls.map(c => c.offset)).toEqual([0, 200]);
    expect(p.rowAt(250)?.faceId).toBe(251);
  });

  it('drops responses from an older query', async () => {
    const api = deferredApi();
    const p = new PagedFaces(api, { text: 'a' });
    p.setQuery({ text: 'b' });
    await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);
    api.resolvers[1]({ total: 1, offset: 0, items: [makeRow(42)] });
    api.resolvers[0]({ total: 9, offset: 0, items: [makeRow(1)] });
    await flush();
    expect(p.getSnapshot().total).toBe(1);
    expect(p.rowAt(0)?.faceId).toBe(43);
  });

  it('debounces text; other filters apply immediately and cancel the pending debounce', async () => {
    const api = createMockLibraryApi(rows);
    const p = new PagedFaces(api, {});
    await flush();
    p.setQuery({ text: 'f' }); p.setQuery({ text: 'fa' });
    await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS - 1);
    expect(api.calls).toHaveLength(1);
    p.setQuery({ text: 'fa', favoritesOnly: true }); // áp dụng ngay, huỷ timer của 'fa'
    await flush();
    expect(api.calls).toHaveLength(2);
    expect(api.calls.at(-1)).toMatchObject({ text: 'fa', favoritesOnly: true, offset: 0 });
    await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);
    expect(api.calls).toHaveLength(2);
  });

  it('surfaces errors, does not refetch a failed page on its own, and retries on retry()', async () => {
    let fail = true;
    const api = { queryFaces: vi.fn(async (q: FaceQuery) => { if (fail) throw new Error('db'); return { total: 1, offset: q.offset ?? 0, items: [makeRow(0)] }; }) };
    const p = new PagedFaces(api, {});
    await flush();
    expect(p.getSnapshot().error).toBe('db');
    p.ensureRange(0, 10); p.ensureRange(0, 10);
    await flush();
    expect(api.queryFaces).toHaveBeenCalledTimes(1); // không vòng lặp lỗi → render → fetch
    fail = false;
    p.retry();
    await flush();
    expect(p.getSnapshot()).toMatchObject({ error: null, total: 1 });
    expect(api.queryFaces).toHaveBeenCalledTimes(2);
  });

  it('disposed_instance_ignores_responses', async () => {
    const api = deferredApi();
    const p = new PagedFaces(api, {});
    const cb = vi.fn(); p.subscribe(cb);
    p.dispose();
    api.resolvers[0]({ total: 5, offset: 0, items: [makeRow(0)] });
    await flush();
    expect(cb).not.toHaveBeenCalled();
    expect(p.getSnapshot().total).toBeNull();
    p.setQuery({ text: 'x' }); p.ensureRange(0, 10);
    await vi.advanceTimersByTimeAsync(SEARCH_DEBOUNCE_MS);
    expect(api.calls).toHaveLength(1);
  });
});
