import { StrictMode } from 'react';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMockLibraryApi, makeRow, type MockApi } from '../api';
import { FontFaceLoader } from '../fontFaceLoader';
import { HOVER_DELAY_MS, LibraryGrid, type LibraryGridProps } from '../LibraryGrid';
import { PreviewBlobs, type BlobDeps } from '../previewBlobs';

const rows = Array.from({ length: 5000 }, (_, i) => makeRow(i));
const flush = () => act(async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); });
/** Chuỗi previewInfo → trang → fetch blob → rAF (setTimeout 0 trong setup) cần vài lượt microtask + timer. */
const settle = async () => {
  for (let i = 0; i < 5; i++) { await flush(); await act(async () => { await new Promise(r => setTimeout(r, 0)); }); }
  await flush();
};

function stubLoader() {
  return { acquire: vi.fn(async (r: { faceIndex: number }) => (r.faceIndex > 0 ? null : 'fa-live')), release: vi.fn(), onScroll: vi.fn() } as unknown as FontFaceLoader & { acquire: ReturnType<typeof vi.fn>; release: ReturnType<typeof vi.fn> };
}
function blobs(status: (url: string) => number = () => 200) {
  const fetched: string[] = []; let n = 0;
  const d: BlobDeps = {
    fetch: async (url) => { fetched.push(url); const s = status(url); return { ok: s === 200, status: s, blob: async () => new Blob(['p']), text: async () => 'no_outlines' }; },
    createObjectURL: () => `blob:${++n}`, revokeObjectURL: () => {},
  };
  return { b: new PreviewBlobs(d), fetched };
}
function ui(api: MockApi, extra: Partial<LibraryGridProps>, deps: { loader: ReturnType<typeof stubLoader>; b: PreviewBlobs }) {
  return <LibraryGrid api={api} query={{}} width={1300} height={800} loader={deps.loader} blobs={deps.b} {...extra} />;
}
async function mount(extra: Partial<LibraryGridProps> = {}, data = rows, status?: (u: string) => number) {
  const api = createMockLibraryApi(data);
  const loader = stubLoader(); const { b, fetched } = blobs(status);
  const r = render(ui(api, extra, { loader, b }));
  await settle();
  return { ...r, api, loader, b, fetched, rerenderWith: (e: Partial<LibraryGridProps>) => r.rerender(ui(api, { ...extra, ...e }, { loader, b })) };
}
const cells = () => screen.getAllByRole('gridcell');

afterEach(() => { vi.useRealTimers(); document.documentElement.classList.remove('dark'); });

it('mask stylesheet tints with the theme foreground; img fallback inverts in dark', () => {
  const css = readFileSync(resolve(__dirname, '../libraryGrid.css'), 'utf8');
  expect(css).toMatch(/\.fa-preview-mask\s*{[^}]*background-color:\s*var\(--foreground\)/);
  expect(css).toMatch(/-webkit-mask-image:\s*var\(--preview-url\)/);
  expect(css).toMatch(/\.dark\s+\.fa-preview-img\s*{[^}]*filter:\s*invert\(1\)/);
});

describe('LibraryGrid', () => {
  it('virtualizes: only a few rows of 5000 are in the DOM', async () => {
    await mount();
    expect(cells().length).toBeGreaterThan(0);
    expect(cells().length).toBeLessThan(100);
    expect(screen.getByRole('grid')).toHaveAttribute('aria-rowcount', String(Math.ceil(5000 / 3)));
  });

  it('mask mode: one fetch per url, blob url in the mask, ready after a frame', async () => {
    const { fetched } = await mount();
    const c = cells()[0];
    expect(c).toHaveAttribute('data-state', 'ready');
    expect(c.querySelector('img')).toBeNull();
    const mask = c.querySelector('[data-mask]') as HTMLElement;
    expect(mask).toHaveClass('fa-preview-mask');
    expect(mask.style.getPropertyValue('--preview-url')).toMatch(/^url\("blob:\d+"\)$/);
    expect(new Set(fetched).size).toBe(fetched.length);
  });

  it('theme_toggle_keeps_preview', async () => {
    const { fetched } = await mount();
    const before = fetched.length;
    document.documentElement.classList.add('dark');
    await settle();
    expect(fetched.length).toBe(before);
    expect(cells()[0]).toHaveAttribute('data-state', 'ready');
  });

  it('text_change_keeps_old_image_until_new_ready and uses a backend generation', async () => {
    const { api, rerenderWith } = await mount({ sampleText: 'Một' });
    const c = () => cells()[0];
    const oldShown = c().getAttribute('data-shown-url')!;
    expect(oldShown).toMatch(/g=0/);
    rerenderWith({ sampleText: 'Hai' });
    expect(c().getAttribute('data-shown-url')).toBe(oldShown); // chưa đổi ảnh ngay
    await settle();
    expect(api.currentGeneration()).toBe(1);
    expect(c().getAttribute('data-shown-url')).toMatch(/g=1/);
    expect(c().getAttribute('data-shown-url')).toContain(encodeURIComponent('Hai'));
    expect(c()).toHaveAttribute('data-state', 'ready');
  });

  it('remount_uses_backend_generation', async () => {
    const api = createMockLibraryApi(rows);
    await api.bumpGeneration(); await api.bumpGeneration(); // backend đã ở thế hệ 2 (phiên trước)
    const loader = stubLoader(); const { b } = blobs();
    const first = render(ui(api, {}, { loader, b })); await settle();
    first.unmount();
    render(ui(api, {}, { loader, b })); await settle();
    expect(cells()[0].getAttribute('data-shown-url')).toMatch(/g=2/);
  });

  it('works_under_strict_mode', async () => {
    const api = createMockLibraryApi(rows);
    const loader = stubLoader(); const { b } = blobs();
    render(<StrictMode>{ui(api, {}, { loader, b })}</StrictMode>);
    await settle();
    expect(cells().length).toBeGreaterThan(0);
    expect(cells()[0]).toHaveAttribute('data-state', 'ready');
    expect(cells()[0].getAttribute('data-shown-url')).toMatch(/g=0/);
  });

  it('error_state_shows_placeholder', async () => {
    await mount({}, rows.slice(0, 3), () => 422);
    const c = cells()[0];
    expect(c).toHaveAttribute('data-state', 'error');
    expect(c).toHaveTextContent('Không có preview');
  });

  it('img fallback mode renders a plain img with the protocol url', async () => {
    const { fetched } = await mount({ previewMode: 'img' }, rows.slice(0, 3));
    const img = cells()[0].querySelector('img.fa-preview-img') as HTMLImageElement;
    expect(img.getAttribute('src')).toMatch(/^\/preview\/0{64}_0_48\.png\?/);
    expect(fetched).toHaveLength(0);
    fireEvent.load(img);
    expect(cells()[0]).toHaveAttribute('data-state', 'ready');
  });

  it('hover loads live font after delay and releases on leave', async () => {
    const { loader } = await mount();
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const c = cells()[0];
    fireEvent.pointerEnter(c);
    expect(loader.acquire).not.toHaveBeenCalled();
    await act(async () => { await vi.advanceTimersByTimeAsync(HOVER_DELAY_MS); });
    expect(loader.acquire).toHaveBeenCalledTimes(1);
    expect(c).toHaveAttribute('data-state', 'live');
    fireEvent.pointerLeave(c);
    expect(loader.release).toHaveBeenCalledTimes(1);
  });

  it('live font renders at the same size as the preview (no jump, no clipping)', async () => {
    await mount({ px: 48 });
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const c = cells()[0];
    fireEvent.pointerEnter(c);
    await act(async () => { await vi.advanceTimersByTimeAsync(HOVER_DELAY_MS); });
    const span = c.querySelector('span[data-live]') as HTMLElement;
    expect(span.style.fontSize).toBe('24px'); // ảnh vẽ ở 48 px trên canvas 2× → 24 css px
  });

  it('hover_on_ttc_face_keeps_preview', async () => {
    const { loader } = await mount({}, [makeRow(0, { faceIndex: 1 })]);
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const c = cells()[0];
    fireEvent.pointerEnter(c);
    await act(async () => { await vi.advanceTimersByTimeAsync(HOVER_DELAY_MS); });
    expect(loader.acquire).not.toHaveBeenCalled();
    expect(c).toHaveAttribute('data-state', 'ready');
  });

  it('shows missing Vietnamese badge only for non-ASCII sample on fonts lacking it', async () => {
    await mount({ sampleText: 'Ăn phở' }, [makeRow(0, { supportsVi: false, viMissing: 12 }), makeRow(1)]);
    const [a, b] = cells();
    expect(a).toHaveTextContent('Thiếu 12 ký tự tiếng Việt');
    expect(b).not.toHaveTextContent('Thiếu');
  });

  it('click and Enter select the face', async () => {
    const onSelect = vi.fn();
    await mount({ onSelect });
    const c = cells()[1];
    fireEvent.click(c);
    fireEvent.keyDown(c, { key: 'Enter' });
    expect(onSelect).toHaveBeenCalledTimes(2);
    expect(onSelect.mock.calls[0][0].faceId).toBe(2);
  });

  it('disposes its own FontFaceLoader on unmount', async () => {
    const spy = vi.spyOn(FontFaceLoader.prototype, 'dispose');
    const api = createMockLibraryApi(rows.slice(0, 3)); const { b } = blobs();
    const r = render(<LibraryGrid api={api} query={{}} width={1300} height={800} blobs={b} />);
    await settle();
    r.unmount();
    expect(spy).toHaveBeenCalledTimes(1);
    spy.mockRestore();
  });

  it('shows the empty state', async () => {
    await mount({ query: { text: 'khong co font nao' } });
    expect(screen.getByText('Không có font nào khớp')).toBeInTheDocument();
  });

  it('recovers when the backend generation moved on elsewhere (410)', async () => {
    const api = createMockLibraryApi(rows);
    const realInfo = api.previewInfo.bind(api);
    let first = true;
    // Trả thế hệ 0 nhưng backend đã sang 1 (ví dụ một lần bump treo của grid cũ tới sau).
    api.previewInfo = async () => { const i = await realInfo(); if (first) { first = false; await api.bumpGeneration(); } return i; };
    const loader = stubLoader(); const { b } = blobs(u => (u.includes('g=0') ? 410 : 200));
    render(ui(api, {}, { loader, b }));
    await settle();
    expect(cells()[0]).toHaveAttribute('data-state', 'ready');
    expect(cells()[0].getAttribute('data-shown-url')).toMatch(/g=1/);
  });

  it('shows an error when preview info cannot be loaded', async () => {
    const api = createMockLibraryApi(rows);
    api.previewInfo = async () => { throw new Error('ipc'); };
    const { b } = blobs();
    render(<LibraryGrid api={api} query={{}} width={1300} height={800} loader={stubLoader()} blobs={b} />);
    await settle();
    expect(screen.getByRole('alert')).toHaveTextContent('Không tải được preview');
  });

  it('a failed page keeps the grid and can be retried', async () => {
    let failing = true;
    const api = createMockLibraryApi(rows, { fail: q => ((q.offset ?? 0) >= 200 && failing ? new Error('bận') : null) });
    const { b } = blobs();
    render(<LibraryGrid api={api} query={{}} width={1300} height={8000} loader={stubLoader()} blobs={b} />); // cao → cần trang 1
    await settle();
    expect(cells().length).toBeGreaterThan(0);
    expect(screen.getByRole('alert')).toHaveTextContent('bận');
    expect(api.calls.filter(c => c.offset === 200)).toHaveLength(1); // không tự fetch lại liên tục
    failing = false;
    fireEvent.click(screen.getByRole('button', { name: 'Thử lại' }));
    await settle();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(api.calls.filter(c => c.offset === 200)).toHaveLength(2);
  });

  it('shows the backend error state', async () => {
    const api = createMockLibraryApi(rows, { fail: () => new Error('db hỏng') });
    const { b } = blobs();
    render(<LibraryGrid api={api} query={{}} width={1300} height={800} loader={stubLoader()} blobs={b} />);
    await settle();
    expect(screen.getByRole('alert')).toHaveTextContent('Lỗi tải thư viện: db hỏng');
  });
});
