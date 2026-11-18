// Bằng chứng điều tra beta.3 (library trống thoáng qua, sidebar chậm). Các test mô tả HÀNH VI HIỆN TẠI để chốt lỗi; sau khi sửa sẽ đảo kỳ vọng.
import { act, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { Sidebar } from '../../../components/sidebar/Sidebar';
import { createMockLibraryApi, makeRow } from '../api';
import { LibraryGrid } from '../LibraryGrid';
import type { FacePage, FaceQuery } from '../types';

const flush = () => act(async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); });
const rows = Array.from({ length: 300 }, (_, i) => makeRow(i));

/** Mô phỏng Library.tsx: `<LibraryGrid key={gridRev:scanVersion}>`; mỗi lần bump quét = đổi key. */
function Harness({ api, rev }: { api: ReturnType<typeof createMockLibraryApi>; rev: number }) {
  return <LibraryGrid key={rev} api={api} query={{}} width={1300} height={800} emptyState={<p data-testid="empty">trống</p>} />;
}

describe('bump quét → LibraryGrid remount', () => {
  it('KHOẢNH KHẮC RỖNG: sau bump, lưới không còn thẻ nào cho tới khi trang 0 mới về', async () => {
    const base = createMockLibraryApi(rows);
    let hold: ((p: FacePage) => void) | null = null;
    let gate = false;
    const api = { ...base, queryFaces: (q: FaceQuery) => gate ? new Promise<FacePage>(r => { hold = r; }) : base.queryFaces(q) } as typeof base;
    const r = render(<Harness api={api} rev={0} />);
    await flush();
    expect(screen.queryAllByRole('gridcell').length).toBeGreaterThan(0);
    gate = true; // truy vấn mới (sau bump) chậm: DB đang bận vì quét
    r.rerender(<Harness api={api} rev={1} />);
    await flush();
    expect(screen.queryAllByRole('gridcell').length).toBe(0); // ← trống: lỗi (1)/(3)
    expect(screen.queryByTestId('empty')).toBeNull(); // không phải ZeroState, chỉ là lưới trắng
    act(() => hold?.({ total: 300, offset: 0, items: rows.slice(0, 200) }));
    await flush();
    expect(screen.queryAllByRole('gridcell').length).toBeGreaterThan(0);
  });

  it('total=0 thoáng qua từ backend (đang ghi dở) → hiện ZeroState "trống" dù không lọc gì', async () => {
    const base = createMockLibraryApi(rows);
    let gate = false;
    const api = { ...base, queryFaces: async (q: FaceQuery) => gate ? ({ total: 0, offset: 0, items: [] } as FacePage) : base.queryFaces(q) } as typeof base;
    const r = render(<Harness api={api} rev={0} />);
    await flush();
    gate = true;
    r.rerender(<Harness api={api} rev={1} />);
    await flush();
    expect(screen.getByTestId('empty')).toBeInTheDocument(); // ← lỗi (1): không có chỗ nào giữ dữ liệu cũ khi nhận 0
  });
});

describe('Sidebar.refresh = Promise.all', () => {
  it('một truy vấn chậm giữ cả thanh bên ở trạng thái chưa có số/Folder/Duplicates', async () => {
    const api = createMockApi();
    let release: (() => void) | null = null;
    const slow = new Promise<void>(r => { release = r; });
    const dup = api.duplicateSummary;
    api.duplicateSummary = async (...a: Parameters<typeof dup>) => { await slow; return dup(...a); };
    render(<Sidebar api={api} pickFolder={async () => null} />);
    await act(async () => { await new Promise(r => setTimeout(r, 20)); });
    // listSources/listCollections/listTags đã xong từ lâu, nhưng chưa có dòng nguồn nào vì setSources chờ cả nhóm.
    expect(screen.queryByRole('button', { name: /Dự án khách/ })).toBeNull(); // ← lỗi (2)
    await act(async () => { release?.(); await new Promise(r => setTimeout(r, 20)); });
    expect(screen.getByRole('button', { name: /Dự án khách/ })).toBeInTheDocument();
  });
});
