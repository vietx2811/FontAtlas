// Beta.3: bump quét không được làm lưới trống / dựng lại, sidebar không chờ truy vấn chậm nhất.
import { act, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { Sidebar } from '../../../components/sidebar/Sidebar';
import { useScanStore } from '../../../store/scan';
import { createMockLibraryApi, makeRow } from '../api';
import { LibraryGrid } from '../LibraryGrid';
import type { FacePage, FaceQuery } from '../types';

const flush = () => act(async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); });
const rows = Array.from({ length: 300 }, (_, i) => makeRow(i));
type Api = ReturnType<typeof createMockLibraryApi>;

/** Giống Library.tsx: key chỉ theo gridRev; bump quét đi qua refreshKey/scanning. */
function Harness({ api, rev, scanning = false }: { api: Api; rev: number; scanning?: boolean }) {
  return <LibraryGrid api={api} query={{}} width={1300} height={800} refreshKey={rev} scanning={scanning} emptyState={<p data-testid="empty">trống</p>} />;
}
const cells = () => screen.queryAllByRole('gridcell');
const gated = (base: Api, handler: (q: FaceQuery) => Promise<FacePage> | null): Api => ({ ...base, queryFaces: (q: FaceQuery) => handler(q) ?? base.queryFaces(q) }) as Api;
const zeroPage = () => Promise.resolve({ total: 0, offset: 0, items: [] } as FacePage);

describe('bump quét → LibraryGrid làm tươi tại chỗ', () => {
  it('không còn khoảnh khắc rỗng: thẻ cũ ở lại trong lúc truy vấn mới chưa về, rồi được thay', async () => {
    const base = createMockLibraryApi(rows);
    let hold: ((p: FacePage) => void) | null = null; let gate = false;
    const api = gated(base, () => (gate ? new Promise<FacePage>(r => { hold = r; }) : null));
    const r = render(<Harness api={api} rev={0} />);
    await flush();
    const before = cells().length;
    expect(before).toBeGreaterThan(0);
    gate = true;
    r.rerender(<Harness api={api} rev={1} scanning />);
    await flush();
    expect(cells().length).toBe(before);
    expect(screen.queryByTestId('empty')).toBeNull();
    act(() => hold?.({ total: 300, offset: 0, items: rows.slice(0, 200) }));
    await flush();
    expect(cells().length).toBeGreaterThan(0);
  });

  it('không remount: phần tử cuộn giữ nguyên qua bump', async () => {
    const api = createMockLibraryApi(rows);
    const r = render(<Harness api={api} rev={0} />);
    await flush();
    const scroller = screen.getByTestId('library-grid-scroller');
    r.rerender(<Harness api={api} rev={1} scanning />);
    await flush();
    expect(screen.getByTestId('library-grid-scroller')).toBe(scroller);
  });

  it('bump trong lúc đang cuộn giữ nguyên vị trí cuộn', async () => {
    const api = createMockLibraryApi(rows);
    const r = render(<Harness api={api} rev={0} />);
    await flush();
    const scroller = screen.getByTestId('library-grid-scroller');
    scroller.scrollTop = 400;
    scroller.dispatchEvent(new Event('scroll'));
    await flush();
    r.rerender(<Harness api={api} rev={1} scanning />);
    await flush();
    expect(screen.getByTestId('library-grid-scroller').scrollTop).toBe(400);
  });

  it('total=0 khi ĐANG quét: giữ dữ liệu cũ tới lần xác nhận thứ hai', async () => {
    const base = createMockLibraryApi(rows);
    let zero = false;
    const api = gated(base, () => (zero ? zeroPage() : null));
    const r = render(<Harness api={api} rev={0} />);
    await flush();
    zero = true;
    r.rerender(<Harness api={api} rev={1} scanning />);
    await flush();
    expect(screen.queryByTestId('empty')).toBeNull();
    expect(cells().length).toBeGreaterThan(0); // lần 1: còn nghi
    r.rerender(<Harness api={api} rev={2} scanning />);
    await flush();
    expect(screen.getByTestId('empty')).toBeInTheDocument(); // lần 2 xác nhận
  });

  it('total=0 khi KHÔNG quét: hiện ZeroState ngay (kết quả rỗng thật, vd đã gỡ nguồn)', async () => {
    const base = createMockLibraryApi(rows);
    let zero = false;
    const api = gated(base, () => (zero ? zeroPage() : null));
    const r = render(<Harness api={api} rev={0} />);
    await flush();
    zero = true;
    r.rerender(<Harness api={api} rev={1} scanning={false} />);
    await flush();
    expect(screen.getByTestId('empty')).toBeInTheDocument();
  });

  it('trang 0 lỗi khi làm tươi: giữ dữ liệu cũ, không báo lỗi', async () => {
    const base = createMockLibraryApi(rows);
    let fail = false;
    const api = gated(base, () => (fail ? Promise.reject(new Error('db busy')) : null));
    const r = render(<Harness api={api} rev={0} />);
    await flush();
    fail = true;
    r.rerender(<Harness api={api} rev={1} scanning />);
    await flush();
    expect(cells().length).toBeGreaterThan(0);
    expect(screen.queryByRole('alert')).toBeNull();
  });
});

describe('trang đầu chưa về', () => {
  it('hiện khung xương cùng kích thước thẻ (không vùng trắng, không ZeroState), rồi thay bằng thẻ thật', async () => {
    const base = createMockLibraryApi(rows);
    let hold: ((p: FacePage) => void) | null = null;
    const api = gated(base, () => new Promise<FacePage>(r => { hold = r; }));
    render(<Harness api={api} rev={0} />);
    await flush();
    const skel = screen.queryAllByRole('gridcell');
    expect(skel.length).toBeGreaterThan(0);
    expect(skel.every(c => c.getAttribute('data-state') === 'loading')).toBe(true);
    expect(screen.queryByTestId('empty')).toBeNull();
    act(() => hold?.({ total: 300, offset: 0, items: rows.slice(0, 200) }));
    await flush();
    expect(cells().some(c => c.getAttribute('data-state') !== 'loading')).toBe(true);
  });
});

describe('quét kết thúc bằng failed (không bump)', () => {
  it('scanning true→false làm tươi một lần với scanning=false → số 0 được xác nhận', async () => {
    const base = createMockLibraryApi(rows);
    let zero = false;
    const api = gated(base, () => (zero ? zeroPage() : null));
    const r = render(<Harness api={api} rev={0} scanning />);
    await flush();
    zero = true;
    r.rerender(<Harness api={api} rev={1} scanning />);
    await flush();
    expect(cells().length).toBeGreaterThan(0); // đang quét: giữ
    r.rerender(<Harness api={api} rev={1} scanning={false} />); // không đổi refreshKey (failed không bump)
    await flush();
    expect(screen.getByTestId('empty')).toBeInTheDocument();
  });
});

describe('Sidebar.refresh độc lập từng nhánh', () => {
  it('yêu cầu nạp đến lúc đang chạy gộp thành đúng 1 lần chạy lại, không xếp chồng', async () => {
    const api = createMockApi();
    const real = api.listSources.bind(api);
    const holds: Array<() => void> = [];
    let calls = 0;
    api.listSources = async () => { calls++; const data = await real(); if (calls === 1) await new Promise<void>(r => holds.push(r)); return data; };
    const r = render(<Sidebar api={api} pickFolder={async () => null} />);
    await act(async () => { await new Promise(x => setTimeout(x, 10)); });
    useScanStore.getState().bump(); useScanStore.getState().bump(); // bump trong lúc nạp đầu: bỏ qua
    await act(async () => { await new Promise(x => setTimeout(x, 10)); });
    expect(calls).toBe(1);
    await act(async () => { holds.forEach(h => h()); await new Promise(x => setTimeout(x, 30)); });
    expect(calls).toBe(1); // bump sớm không kích lần nạp thứ hai
    expect(screen.getByRole('button', { name: /Dự án khách/ })).toBeInTheDocument();
    // sau khi đã nạp xong: mỗi đợt bump (kể cả nhiều bump cùng một lượt vẽ) cho đúng 1 lần nạp
    calls = 1; holds.length = 0;
    act(() => { useScanStore.getState().bump(); });
    await act(async () => { await new Promise(x => setTimeout(x, 10)); });
    act(() => { useScanStore.getState().bump(); useScanStore.getState().bump(); });
    await act(async () => { await new Promise(x => setTimeout(x, 30)); });
    expect(calls).toBe(3); // 1 + 1 (bump đơn) + 1 (hai bump cùng lượt)
    r.unmount();
  });

  it('phản hồi về sau unmount không set state (không cảnh báo)', async () => {
    const api = createMockApi();
    let release: (() => void) | null = null;
    const real = api.listTags.bind(api);
    api.listTags = async () => { await new Promise<void>(x => { release = x; }); return real(); };
    const err = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    const r = render(<Sidebar api={api} pickFolder={async () => null} />);
    await act(async () => { await new Promise(x => setTimeout(x, 10)); });
    r.unmount();
    await act(async () => { release?.(); await new Promise(x => setTimeout(x, 10)); });
    expect(err).not.toHaveBeenCalled();
    err.mockRestore();
  });

  it('đếm trùng chậm không giữ Folder; Duplicates hiện "…" thay vì 0 rồi ra số', async () => {
    const api = createMockApi();
    let release: (() => void) | null = null;
    const slow = new Promise<void>(r => { release = r; });
    const dup = api.duplicateSummary;
    api.duplicateSummary = async (...a: Parameters<typeof dup>) => { await slow; return dup(...a); };
    render(<Sidebar api={api} pickFolder={async () => null} />);
    await act(async () => { await new Promise(r => setTimeout(r, 20)); });
    expect(screen.getByRole('button', { name: /Dự án khách/ })).toBeInTheDocument(); // nguồn có ngay
    expect(screen.getAllByText('…').length).toBeGreaterThan(0);
    await act(async () => { release?.(); await new Promise(r => setTimeout(r, 20)); });
    expect(screen.queryByText('…')).toBeNull();
  });

  it('một nhánh lỗi không chặn các nhánh khác', async () => {
    const api = createMockApi();
    api.listCases = async () => { throw new Error('boom'); };
    render(<Sidebar api={api} pickFolder={async () => null} />);
    await act(async () => { await new Promise(r => setTimeout(r, 20)); });
    expect(screen.getByRole('button', { name: /Dự án khách/ })).toBeInTheDocument();
  });
});
