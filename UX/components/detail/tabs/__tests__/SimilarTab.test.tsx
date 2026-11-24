import { act, fireEvent, render, screen, within, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../../api/mock';
import type { FaceDetailDto } from '../../../../api';
import { faceKey, useUiStore } from '../../../../store/ui';
import { createModelStore } from '../../../../store/model';
import { SOURCE_BADGE, SimilarTab } from '../SimilarTab';

const initial = useUiStore.getState();
async function detail(family = 'Roboto'): Promise<FaceDetailDto> {
  const api = createMockApi();
  const r = (await api.queryFaces({ text: family, limit: 1 })).items[0];
  return api.getFace({ hashHex: r.hashHex, faceIndex: 0 }, 'vi');
}

describe('SimilarTab', () => {
  beforeEach(() => useUiStore.setState(initial, true));

  it('loads one result per other family, explains it without a score and pages by family', async () => {
    const api = createMockApi();
    const d = await detail();
    const spy = vi.spyOn(api, 'similarFamilies');
    render(<SimilarTab api={api} detail={d} />);
    const items = await screen.findAllByTestId('similar-hit');
    expect(items).toHaveLength(6);
    const first = spy.mock.results[0];
    const page = await first.value;
    expect(new Set(page.items.map(g => g.family)).size).toBe(page.items.length);
    expect(page.items.every(g => g.family !== d.row.family)).toBe(true);
    for (const item of items) {
      expect(item.textContent).toMatch(/Giống: .+/);
      expect(item.textContent).toMatch(/Khác: .+/);
      expect(within(item).getByTestId('source-badge')).toBeTruthy();
      expect(item.textContent).not.toMatch(/x_height|stem_weight|%/);
    }
    const firstNames = page.items.map(g => g.family);
    fireEvent.click(screen.getByRole('button', { name: 'Trang sau' }));
    await waitFor(() => expect(spy).toHaveBeenLastCalledWith(expect.anything(), expect.anything(), expect.objectContaining({ offset: 6, limit: 6 })));
    const second = await spy.mock.results.at(-1)!.value;
    await screen.findAllByTestId('similar-hit');
    expect(second.items.every(g => !firstNames.includes(g.family))).toBe(true);
    expect(second.total).toBe(page.total);
  });

  it('Mở font opens the representative specimen; Xem family uses backend members and closes detail', async () => {
    const api = createMockApi(); const d = await detail();
    const page = await api.similarFamilies(d.row, {}, { limit: 6 });
    render(<SimilarTab api={api} detail={d} />);
    const items = await screen.findAllByTestId('similar-hit');
    fireEvent.click(within(items[0]).getByRole('button', { name: 'Mở font' }));
    expect(useUiStore.getState().selectedId).toBe(faceKey(page.items[0].representative.face));
    expect(useUiStore.getState().panelTab).toBe('spec');
    fireEvent.click(within(items[0]).getByRole('button', { name: 'Xem family' }));
    await waitFor(() => expect(useUiStore.getState().familyScope).toEqual({ name: page.items[0].family, faces: page.items[0].faces }));
    expect(useUiStore.getState().selectedId).toBeNull();
    const { toGridQuery } = await import('../../../../store/ui');
    expect(toGridQuery(useUiStore.getState()).faceKeys).toEqual(page.items[0].faces);
    act(() => useUiStore.getState().setView('all'));
    expect(useUiStore.getState().familyScope).toBeNull();
    expect(toGridQuery(useUiStore.getState()).faceKeys).toBeUndefined();
  });

  it('sends real filters, resets offset when scope changes and offers retry/empty states', async () => {
    const api = createMockApi(); const d = await detail();
    const spy = vi.spyOn(api, 'similarFamilies');
    render(<SimilarTab api={api} detail={d} />);
    await screen.findAllByTestId('similar-hit');
    fireEvent.click(screen.getByRole('button', { name: 'Trang sau' }));
    await screen.findAllByTestId('similar-hit');
    act(() => useUiStore.getState().setFilters({ italic: true, rawAxisRanges: [{ axis: 'slant_deg', range: [0, 30] }] }));
    await waitFor(() => expect(spy).toHaveBeenLastCalledWith(expect.anything(), expect.anything(), expect.objectContaining({ offset: 0, italic: true, rawAxisRanges: [{ axis: 'slant_deg', range: [0, 30] }] })));
    spy.mockRejectedValueOnce(new Error('index unavailable'));
    act(() => useUiStore.getState().setQuery('test'));
    expect(await screen.findByRole('alert')).toHaveTextContent('index unavailable');
    spy.mockResolvedValueOnce({ total: 0, offset: 0, items: [] });
    fireEvent.click(screen.getByRole('button', { name: 'Thử lại' }));
    expect(await screen.findByTestId('zero-state')).toHaveTextContent('Không có family tương tự');
  });

  it('discovers other families inside a family scope and ignores recent/active membership changes', async () => {
    const api = createMockApi(); const d = await detail('Inter');
    const styles = await api.getFamilyStyles('Inter');
    useUiStore.getState().openFamily({ name: 'Inter', faces: styles.map(s => s.face) });
    const spy = vi.spyOn(api, 'similarFamilies');
    render(<SimilarTab api={api} detail={d} />);
    await screen.findAllByTestId('similar-hit');
    expect(spy.mock.calls[0][2]).not.toHaveProperty('faceKeys');
    expect((await spy.mock.results[0].value).items.every(g => g.family !== 'Inter')).toBe(true);
    act(() => { useUiStore.getState().setView('recent'); useUiStore.getState().select('another:0'); });
    await act(async () => {});
    act(() => { useUiStore.getState().setView('active'); useUiStore.setState({ active: new Map([['different', [0]]]) }); });
    await act(async () => {});
    expect(spy).toHaveBeenCalledTimes(1);
    act(() => useUiStore.getState().setFilters({ italic: true }));
    await waitFor(() => expect(spy).toHaveBeenCalledTimes(2));
    expect(spy.mock.calls[1][2]).toMatchObject({ italic: true, offset: 0 });
    expect(spy.mock.calls[1][2]).not.toHaveProperty('faceKeys');
  });

  it('resets offset permanently on a scope round trip and recovers after total shrinks', async () => {
    const api = createMockApi(); const d = await detail();
    const original = api.similarFamilies.bind(api);
    let shrink = false;
    const spy = vi.spyOn(api, 'similarFamilies').mockImplementation(async (face, opts, q = {}) => {
      const p = await original(face, opts, q);
      if (shrink) return { ...p, total: 3, items: q.offset ? [] : p.items.slice(0, 3) };
      return p;
    });
    render(<SimilarTab api={api} detail={d} />);
    await screen.findAllByTestId('similar-hit');
    fireEvent.click(screen.getByRole('button', { name: 'Trang sau' }));
    await waitFor(() => expect(screen.getByText('7–12 / 18')).toBeTruthy());
    act(() => useUiStore.getState().setQuery('Inter'));
    await waitFor(() => expect(spy.mock.calls.at(-1)![2]).toMatchObject({ text: 'Inter', offset: 0 }));
    act(() => useUiStore.getState().setQuery(''));
    await waitFor(() => expect(screen.getByText('1–6 / 18')).toBeTruthy());
    shrink = true;
    fireEvent.click(screen.getByRole('button', { name: 'Trang sau' }));
    await waitFor(() => expect(screen.getByText('1–3 / 3')).toBeTruthy());
    expect(spy.mock.calls.at(-1)![2]).toMatchObject({ offset: 0 });
    expect(screen.getByRole('button', { name: 'Trang sau' })).toBeDisabled();
  });

  it('opens all family members without filtering so clearing filters reveals the missing styles', async () => {
    const api = createMockApi(); const d = await detail();
    useUiStore.getState().setFilters({ italic: true });
    const family = vi.spyOn(api, 'getFamilyStyles');
    render(<SimilarTab api={api} detail={d} />);
    const items = await screen.findAllByTestId('similar-hit');
    fireEvent.click(within(items[0]).getByRole('button', { name: 'Xem family' }));
    await waitFor(() => expect(useUiStore.getState().familyScope).not.toBeNull());
    const s = useUiStore.getState();
    expect(family).toHaveBeenCalledWith(s.familyScope!.name);
    const { toGridQuery } = await import('../../../../store/ui');
    const filtered = await api.queryFaces(toGridQuery(s));
    expect(filtered.total).toBeLessThan(s.familyScope!.faces.length);
    act(() => s.setFilters({}));
    const all = await api.queryFaces(toGridQuery(useUiStore.getState()));
    expect(all.total).toBe(s.familyScope!.faces.length);
  });

  it('does not silently truncate a family over the 2000 face query limit', async () => {
    const api = createMockApi(); const d = await detail();
    const p = await api.similarFamilies(d.row, {}, { limit: 1 });
    p.items[0].count = 2001;
    vi.spyOn(api, 'similarFamilies').mockResolvedValue(p);
    render(<SimilarTab api={api} detail={d} />);
    await screen.findAllByTestId('similar-hit');
    const xf = screen.getByRole('button', { name: 'Xem family' });
    expect(xf).toHaveAttribute('aria-disabled', 'true'); // vẫn focus được để tooltip lý do hiện bằng bàn phím
    fireEvent.click(xf);
    expect(useUiStore.getState().familyScope).toBeNull();
    expect(screen.getByRole('button', { name: 'Mở font' })).not.toBeDisabled();
    expect(screen.getByText(/Hơn 2.000 kiểu/)).toBeTruthy();
  });

  it('badges follow the source', () => {
    expect(SOURCE_BADGE).toEqual({ axes: 'Số đo', axes_and_image: 'Số đo + Hình ảnh', image: 'Hình ảnh' });
  });

  describe('image-analysis strip (UX 03 §3.3)', () => {
    const settle = async () => { for (let i = 0; i < 5; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
    async function strip(init: Parameters<typeof createMockApi>[0] = {}, embedEnabled = false) {
      const api = createMockApi(init);
      if (embedEnabled) await api.setSettings({ embedEnabled: true });
      const store = createModelStore();
      await act(async () => { await store.getState().sync(api); });
      render(<SimilarTab api={api} detail={await detail()} store={store} />);
      await settle();
      return { api, store };
    }

    it('E0/E1: explains the measures-only result and offers [Bật]; [Bật] enables and starts the download', async () => {
      const { store } = await strip();
      const note = screen.getByTestId('image-strip');
      expect(note.textContent).toMatch(/Kết quả chỉ dựa trên số đo giải phẫu/);
      expect(note.textContent).toMatch(/371 MB/);
      await act(async () => { fireEvent.click(within(note).getByRole('button', { name: 'Bật' })); });
      expect(store.getState().ui().code).toBe('E2');
    });

    it('[Không hiện lại] hides the strip and remembers it in ui.similar_hint_off', async () => {
      const { api } = await strip();
      await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Không hiện lại' })); });
      expect(screen.queryByTestId('image-strip')).toBeNull();
      expect(await api.getSetting('ui.similar_hint_off')).toBe('1');
    });

    it('E2: says the download is running and results are measures-only for now', async () => {
      await strip({ initial: { model: { state: 'downloading', doneBytes: 140 * 1024 * 1024 } } }, true);
      expect(screen.getByTestId('image-strip').textContent).toMatch(/Đang tải model 38%.*Kết quả hiện chỉ dựa trên số đo/);
    });

    it('E4–E7: the error with its retry action', async () => {
      await strip({ initial: { model: { state: 'checksum_mismatch' } } }, true);
      const note = screen.getByTestId('image-strip');
      expect(note.textContent).toMatch(/Model tải về bị hỏng/);
      expect(within(note).getByRole('button', { name: 'Tải lại model' })).toBeTruthy();
    });

    it('E8: "Đang phân tích hình ảnh 412/3.800 — kết quả sẽ đầy đủ hơn khi xong"', async () => {
      await strip({ initial: { model: { state: 'ready' }, embed: { enabled: true, running: true, done: 412, pending: 3388 } } }, true);
      expect(screen.getByTestId('image-strip').textContent).toMatch(/Đang phân tích hình ảnh 412\/3\.800 — kết quả sẽ đầy đủ hơn khi xong/);
    });

    it('ready and idle: no strip', async () => {
      await strip({ initial: { model: { state: 'ready' }, embed: { enabled: true, done: 3800 } } }, true);
      expect(screen.queryByTestId('image-strip')).toBeNull();
    });
  });
});
