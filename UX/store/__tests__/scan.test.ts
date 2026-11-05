import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../api/mock';
import { useNoticeStore } from '../notices';
import { BUMP_THROTTLE_MS, createScanStore, startScanSync } from '../scan';

beforeEach(() => { vi.useFakeTimers(); useNoticeStore.setState({ notices: [] }); });
afterEach(() => vi.useRealTimers());
const run = (ms: number) => vi.advanceTimersByTimeAsync(ms);

describe('scan store', () => {
  it('follows a scan: scanning on, faces written, then off with an immediate refresh at the end', async () => {
    const api = createMockApi({ tickMs: 100 });
    const s = createScanStore();
    const stop = await startScanSync(api, s);
    expect(s.getState().scanning).toBe(false);
    const v0 = s.getState().version;
    await api.addSource('/Users/me/NewFonts');
    await run(150);
    expect(s.getState().scanning).toBe(true);
    expect(s.getState().written).toBeGreaterThan(0);
    await run(500);
    expect(s.getState().scanning).toBe(false);
    expect(s.getState().version).toBeGreaterThan(v0);
    stop();
  });

  it('refreshes the library while faces arrive, but at most once per throttle window', async () => {
    const api = createMockApi({ tickMs: 100 });
    const s = createScanStore();
    await startScanSync(api, s);
    await api.addSource('/Users/me/NewFonts');
    const seen: number[] = [];
    s.subscribe(st => seen.push(st.version));
    await run(450); // 4 facesAdded trong 400 ms
    const distinct = new Set(seen).size;
    expect(distinct).toBeGreaterThanOrEqual(1);
    expect(distinct).toBeLessThanOrEqual(1 + Math.ceil(450 / BUMP_THROTTLE_MS) + 1);
  });

  it('a scan already running when the app opens is picked up from scan_status', async () => {
    const api = createMockApi();
    vi.spyOn(api, 'scanStatus').mockResolvedValue({ running: true, paused: false, currentSourceId: 1, queued: [] });
    const s = createScanStore();
    await startScanSync(api, s);
    expect(s.getState().scanning).toBe(true);
  });

  it('a failed source scan shows an error with its code', async () => {
    const api = createMockApi();
    let push: (e: import('../../api').ScanEvent) => void = () => {};
    vi.spyOn(api, 'scanSubscribe').mockImplementation(async l => { push = l; return () => {}; });
    const s = createScanStore();
    await startScanSync(api, s);
    push({ type: 'failed', sourceId: 2, error: { code: 'io', message: 'không đọc được thư mục', detail: null } });
    const n = useNoticeStore.getState().notices;
    expect(n[0].kind).toBe('error');
    expect(n[0].text).toMatch(/io.*không đọc được thư mục/);
    expect(s.getState().scanning).toBe(false);
  });

  it('stop() unsubscribes', async () => {
    const api = createMockApi({ tickMs: 100 });
    const s = createScanStore();
    const stop = await startScanSync(api, s);
    stop();
    await api.addSource('/Users/me/NewFonts');
    await run(600);
    expect(s.getState().scanning).toBe(false);
  });

  it('counts files skipped as not downloaded from progress events, per source, and clears them when the source finishes (the count then lives in its last scan)', async () => {
    const api = createMockApi();
    let push: (e: import('../../api').ScanEvent) => void = () => {};
    vi.spyOn(api, 'scanSubscribe').mockImplementation(async l => { push = l; return () => {}; });
    const s = createScanStore();
    await startScanSync(api, s);
    const prog = (id: number, nd: number) => ({ type: 'progress' as const, sourceId: id, progress: { phase: 'walking' as const, done: 1, total: 9, facesWritten: 0, notDownloaded: nd } });
    push(prog(1, 3)); push(prog(2, 4));
    expect(s.getState().cloud).toEqual({ 1: 3, 2: 4 });
    expect(s.getState().cloudTotal()).toBe(7);
    push(prog(1, 5));
    expect(s.getState().cloudTotal()).toBe(9);
    push({ type: 'finished', sourceId: 1, stats: { status: 'ok', seen: 9, unchanged: 0, notDownloaded: 5, relinked: 0, analyzed: 0, failed: 0, missing: 0, unreadable: [] } });
    expect(s.getState().cloud).toEqual({ 2: 4 });
    push({ type: 'queued', sourceId: 2 });
    expect(s.getState().cloud).toEqual({}); // quét lại: đếm lại từ đầu
  });
});
