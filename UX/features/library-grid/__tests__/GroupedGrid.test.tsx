import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockLibraryApi, makeRow, type MockApi } from '../api';
import type { FontFaceLoader } from '../fontFaceLoader';
import { LibraryGrid, type LibraryGridProps } from '../LibraryGrid';
import { PreviewBlobs, type BlobDeps } from '../previewBlobs';

const data = [
  ...Array.from({ length: 5 }, (_, i) => makeRow(i, { family: 'Alpha', weightClass: 100 * (i + 1) })),
  makeRow(5, { family: 'Beta' }),
  ...Array.from({ length: 4 }, (_, i) => makeRow(6 + i, { family: 'Gamma', weightClass: 100 * (i + 1) })),
];
const flush = () => act(async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); });
const settle = async () => { for (let i = 0; i < 5; i++) { await flush(); await act(async () => { await new Promise(r => setTimeout(r, 0)); }); } await flush(); };
const loader = () => ({ acquire: vi.fn(async () => 'fa-live'), release: vi.fn(), onScroll: vi.fn() }) as unknown as FontFaceLoader;
const blobs = () => { const d: BlobDeps = { fetch: async () => ({ ok: true, status: 200, blob: async () => new Blob(['p']), text: async () => '' }), createObjectURL: () => 'blob:1', revokeObjectURL: () => {} }; return new PreviewBlobs(d); };
async function mount(extra: Partial<LibraryGridProps> = {}, api: MockApi = createMockLibraryApi(data)) {
  const l = loader(); const b = blobs();
  const el = (e: Partial<LibraryGridProps>) => <LibraryGrid api={api} query={{ groupByFamily: true }} width={1300} height={800} loader={l} blobs={b} {...extra} {...e} />;
  const r = render(el({})); await settle();
  return { ...r, api, rerenderWith: async (e: Partial<LibraryGridProps>) => { r.rerender(el(e)); await settle(); } };
}
const heads = () => screen.queryAllByTestId('group-head');
const names = () => screen.queryAllByTestId('face-name');
afterEach(cleanup);

describe('grid grouped by Family (R3)', () => {
  it('shows one header per family with the style count, then that family\'s cards; every face once', async () => {
    await mount();
    expect(heads().map(h => h.dataset.family)).toEqual(['Alpha', 'Beta', 'Gamma']);
    expect(screen.getAllByTestId('group-count').map(c => c.textContent)).toEqual(['5 kiểu', '1 kiểu', '4 kiểu']);
    expect(names()).toHaveLength(10);
    expect(screen.getByRole('grid').dataset.grouped).toBe('true');
  });
  it('clicking a header collapses only that family (aria-expanded), clicking again reopens it', async () => {
    await mount();
    expect(heads()[0].getAttribute('aria-expanded')).toBe('true');
    fireEvent.click(heads()[0]); await settle();
    expect(heads()[0].getAttribute('aria-expanded')).toBe('false');
    expect(names()).toHaveLength(5);
    expect(heads()).toHaveLength(3); // tiêu đề nhóm thu gọn vẫn hiện
    fireEvent.click(heads()[0]); await settle();
    expect(names()).toHaveLength(10);
  });
  it('← collapses and → opens the focused header; wrong-direction keys do nothing', async () => {
    await mount();
    const h = () => heads()[1];
    h().focus();
    fireEvent.keyDown(h(), { key: 'ArrowRight' }); await settle();
    expect(h().getAttribute('aria-expanded')).toBe('true');
    fireEvent.keyDown(h(), { key: 'ArrowLeft' }); await settle();
    expect(h().getAttribute('aria-expanded')).toBe('false');
    fireEvent.keyDown(h(), { key: 'ArrowLeft' }); await settle();
    expect(h().getAttribute('aria-expanded')).toBe('false');
    fireEvent.keyDown(h(), { key: 'ArrowRight' }); await settle();
    expect(h().getAttribute('aria-expanded')).toBe('true');
  });
  it('a different query opens every group again (collapse lives only for the query)', async () => {
    const m = await mount();
    fireEvent.click(heads()[2]); await settle();
    expect(names()).toHaveLength(6);
    await m.rerenderWith({ query: { groupByFamily: true, text: 'a' } });
    await m.rerenderWith({ query: { groupByFamily: true } });
    expect(names()).toHaveLength(10);
  });
  it('reports the face ranges of collapsed groups (so selection can skip them) and clears them when reopened', async () => {
    const onHiddenRanges = vi.fn();
    await mount({ onHiddenRanges });
    expect(onHiddenRanges).toHaveBeenLastCalledWith([]);
    fireEvent.click(heads()[0]); await settle();
    expect(onHiddenRanges).toHaveBeenLastCalledWith([[0, 4]]);
    fireEvent.click(heads()[2]); await settle();
    expect(onHiddenRanges).toHaveBeenLastCalledWith([[0, 4], [6, 9]]);
    fireEvent.click(heads()[0]); fireEvent.click(heads()[2]); await settle();
    expect(onHiddenRanges).toHaveBeenLastCalledWith([]);
  });
  it('same total, new sort: stale group index is never combined with pages of another query (flat until both match, then new order)', async () => {
    const api = createMockLibraryApi(data);
    const realQ = api.queryFaces.bind(api); const realG = api.familyGroups!.bind(api);
    api.queryFaces = async q => { const all = (await realQ({ ...q, offset: 0, limit: 1000 })).items; const arr = q.descending ? [...all].reverse() : all; const off = q.offset ?? 0; return { total: arr.length, offset: off, items: arr.slice(off, off + (q.limit ?? 100)) }; };
    let release: (() => void) | null = null;
    api.familyGroups = async q => {
      const g = await realG(q);
      if (!q.descending) return g;
      await new Promise<void>(r => { release = r; }); // chỉ mục của truy vấn mới về chậm
      let at = 0; return [...g].reverse().map(x => { const o = { ...x, start: at }; at += x.count; return o; });
    };
    const m = await mount({}, api);
    expect(heads().map(h => h.dataset.family)).toEqual(['Alpha', 'Beta', 'Gamma']);
    await m.rerenderWith({ query: { groupByFamily: true, sort: 'family', descending: true } });
    expect(heads()).toHaveLength(0);                 // truy vấn mới, chỉ mục chưa về → phẳng, không dùng nhóm cũ
    expect(screen.getByRole('grid').dataset.grouped).toBeUndefined();
    await act(async () => { release?.(); }); await settle();
    expect(heads().map(h => h.dataset.family)).toEqual(['Gamma', 'Beta', 'Alpha']);
    expect(names()).toHaveLength(10);
  });
  it('headers are ONE tab stop (roving tabindex): ↓/↑ move between headers and the stop follows the focus', async () => {
    await mount();
    const stops = () => heads().filter(h => h.tabIndex === 0).map(h => h.dataset.family);
    expect(stops()).toEqual(['Alpha']);
    heads()[0].focus();
    fireEvent.keyDown(heads()[0], { key: 'ArrowDown' }); await settle();
    expect(document.activeElement).toBe(heads()[1]);
    expect(stops()).toEqual(['Beta']);
    fireEvent.keyDown(heads()[1], { key: 'ArrowDown' }); await settle();
    expect(document.activeElement).toBe(heads()[2]);
    fireEvent.keyDown(heads()[2], { key: 'ArrowDown' }); await settle(); // đã ở cuối: đứng yên
    expect(document.activeElement).toBe(heads()[2]);
    fireEvent.keyDown(heads()[2], { key: 'ArrowUp' }); await settle(); fireEvent.keyDown(heads()[1], { key: 'ArrowUp' }); await settle();
    expect(document.activeElement).toBe(heads()[0]);
    expect(stops()).toEqual(['Alpha']);
  });
  it('controlled collapse: the grid follows `collapsed` and reports the toggled group (with its range) instead of keeping its own state', async () => {
    const onToggleGroup = vi.fn();
    const m = await mount({ collapsed: new Set(['Beta']), onToggleGroup });
    expect(heads()[1].getAttribute('aria-expanded')).toBe('false');
    expect(names()).toHaveLength(9);
    fireEvent.click(heads()[0]); await settle();
    expect(onToggleGroup).toHaveBeenCalledWith(expect.objectContaining({ family: 'Alpha', start: 0, count: 5 }), true);
    expect(heads()[0].getAttribute('aria-expanded')).toBe('true'); // controlled: nothing changes until the owner says so
    await m.rerenderWith({ collapsed: new Set(['Alpha', 'Beta']), onToggleGroup });
    expect(names()).toHaveLength(4);
  });
  it('the count says "m/N kiểu" when filters hide some styles of the family', async () => {
    const api = createMockLibraryApi(data);
    const real = api.familyGroups!.bind(api);
    api.familyGroups = async q => (await real(q)).map(g => (g.family === 'Alpha' ? { ...g, styleTotal: 9 } : g));
    await mount({}, api);
    const c = screen.getAllByTestId('group-count');
    expect(c[0].textContent).toBe('5/9 kiểu');
    expect(c[0].dataset.filtered).toBe('true');
    expect(c[1].dataset.filtered).toBeUndefined();
  });
  it('without groupByFamily the grid is flat: no headers', async () => {
    await mount({ query: {} });
    expect(heads()).toHaveLength(0);
    expect(names()).toHaveLength(10);
    expect(screen.getByRole('grid').dataset.grouped).toBeUndefined();
  });
  it('a failing or inconsistent group index falls back to the flat grid instead of breaking', async () => {
    const bad = createMockLibraryApi(data);
    bad.familyGroups = async () => { throw new Error('boom'); };
    await mount({}, bad);
    expect(heads()).toHaveLength(0); expect(names()).toHaveLength(10);
    cleanup();
    const off = createMockLibraryApi(data);
    off.familyGroups = async q => (await createMockLibraryApi(data).familyGroups!(q)).slice(0, 2); // không phủ đủ
    await mount({}, off);
    expect(heads()).toHaveLength(0); expect(names()).toHaveLength(10);
  });
  it('picking a card reports its index in the query order (selection ranges keep working)', async () => {
    const onPick = vi.fn();
    await mount({ onPick });
    const gamma = names().find(n => n.textContent?.includes('Gamma'))!;
    fireEvent.click(gamma.closest('[role="gridcell"]')!); await settle();
    expect(onPick).toHaveBeenCalled();
    const [row, index] = onPick.mock.calls[0];
    expect(row.family).toBe('Gamma');
    expect(index).toBe(row.faceId - 1); // faceId = i+1, và nhóm giữ thứ tự truy vấn
  });
});
