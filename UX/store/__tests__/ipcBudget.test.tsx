// Ngân sách lệnh api: khởi động và một lượt quét không đổi gì không được làm lệnh nhân lên (mỗi lệnh là một IPC tranh khoá DB ở bản thật).
import { act, render } from '@testing-library/react';
import { StrictMode } from 'react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../api/mock';
import type { Api, ScanEvent } from '../../api';
import { withInflightDedupe } from '../../api/inflight';
import { Sidebar } from '../../components/sidebar/Sidebar';
import { TopBar } from '../../components/main/TopBar';
import { Library } from '../../components/main/Library';
import { startScanSync, useScanStore } from '../scan';
import { useUiStore } from '../ui';

const initial = useUiStore.getState();
const settle = async (ms = 30) => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, ms / 6)); }); };

function counting(): { api: Api; counts: Record<string, number>; reset(): void; emit(e: ScanEvent): void } {
  const base = createMockApi();
  const counts: Record<string, number> = {};
  let handler: ((e: ScanEvent) => void) | null = null;
  const counted = new Proxy(base, {
    get(t, k, r) {
      const v = Reflect.get(t, k, r);
      if (k === 'scanSubscribe') return async (cb: (e: ScanEvent) => void) => { handler = cb; return () => { handler = null; }; };
      if (typeof v !== 'function') return v;
      return (...a: unknown[]) => { counts[String(k)] = (counts[String(k)] ?? 0) + 1; return (v as (...x: unknown[]) => unknown).apply(t, a); };
    },
  }) as Api;
  const api = withInflightDedupe(counted); // như index.tsx: đếm các IPC THẬT, dưới lớp gộp
  return { api, counts, reset: () => { for (const k of Object.keys(counts)) delete counts[k]; }, emit: e => handler?.(e) };
}
const budget = (c: Record<string, number>) => ({ query_faces: c.queryFaces ?? 0, list_sources: c.listSources ?? 0, duplicate_summary: c.duplicateSummary ?? 0 });

describe('ngân sách lệnh api', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useScanStore.setState({ version: 0, scanning: false, written: 0 }); });

  it('khởi động (StrictMode)', async () => {
    const h = counting();
    render(<StrictMode><div><Sidebar api={h.api} pickFolder={async () => null} /><TopBar api={h.api} /><Library api={h.api} width={1200} height={700} /></div></StrictMode>);
    await settle(200);
    console.log('STARTUP', JSON.stringify(budget(h.counts)), JSON.stringify(h.counts));
    expect(budget(h.counts).list_sources).toBeLessThanOrEqual(3);
    expect(budget(h.counts).query_faces).toBeLessThanOrEqual(6);
  });

  it('một lượt quét không đổi gì (queued → finished → idle) qua đúng store/scan.ts', async () => {
    const h = counting();
    const stop = await startScanSync(h.api); // store toàn cục, như App
    render(<div><Sidebar api={h.api} pickFolder={async () => null} /><TopBar api={h.api} /><Library api={h.api} width={1200} height={700} /></div>);
    await settle(200);
    h.reset();
    const stats = { status: 'ok', seen: 9, unchanged: 9, notDownloaded: 0, relinked: 0, analyzed: 0, failed: 0, missing: 0, purged: 0, unreadable: [] } as never;
    act(() => { h.emit({ type: 'queued', sourceId: 1, trigger: 'user' } as ScanEvent); });
    act(() => { h.emit({ type: 'finished', sourceId: 1, stats, changed: true, trigger: 'user' } as ScanEvent); });
    act(() => { h.emit({ type: 'idle' } as ScanEvent); }); // liền sau finished: gộp
    await settle(300);
    console.log('SCAN_PASS', JSON.stringify(budget(h.counts)), JSON.stringify(h.counts));
    expect(budget(h.counts).query_faces).toBeLessThanOrEqual(5);
    expect(budget(h.counts).list_sources).toBeLessThanOrEqual(2);
    expect(budget(h.counts).duplicate_summary).toBeLessThanOrEqual(1);
    stop();
  });

  const pass = async (events: (emit: (e: object) => void) => void) => {
    const h = counting();
    const stop = await startScanSync(h.api);
    render(<div><Sidebar api={h.api} pickFolder={async () => null} /><TopBar api={h.api} /><Library api={h.api} width={1200} height={700} /></div>);
    await settle(200);
    h.reset();
    act(() => { events(e => h.emit(e as ScanEvent)); });
    await settle(400);
    const b = budget(h.counts);
    stop();
    return b;
  };
  const okStats = (o: object = {}) => ({ status: 'ok', seen: 9, unchanged: 0, notDownloaded: 0, relinked: 0, analyzed: 9, failed: 0, missing: 0, purged: 0, unreadable: [], ...o });

  it('một lượt có facesAdded rồi finished(changed) rồi idle: ≤3 query_faces (một lần refresh)', async () => {
    const b = await pass(emit => {
      emit({ type: 'queued', sourceId: 1, trigger: 'user' });
      emit({ type: 'facesAdded', sourceId: 1, count: 3, trigger: 'user' });
      emit({ type: 'finished', sourceId: 1, stats: okStats(), changed: true, trigger: 'user' });
      emit({ type: 'idle' });
    });
    expect(b.query_faces).toBeLessThanOrEqual(3);
    expect(b.list_sources).toBeLessThanOrEqual(1);
  });

  it('lượt changed=false: 0 query_faces, 0 list_sources', async () => {
    const b = await pass(emit => {
      emit({ type: 'queued', sourceId: 1, trigger: 'user' });
      emit({ type: 'finished', sourceId: 1, stats: okStats({ analyzed: 0, unchanged: 9 }), changed: false, trigger: 'user' });
      emit({ type: 'idle' });
    });
    expect(b.query_faces).toBe(0);
    expect(b.list_sources).toBe(0);
  });
});
