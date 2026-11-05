import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
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

  it('shows the 3 nearest fonts, each with "Giống ở … · Khác ở …" and a source badge, and no score', async () => {
    const d = await detail();
    const { container } = render(<SimilarTab api={createMockApi()} detail={d} />);
    const items = screen.getAllByTestId('similar-hit');
    expect(items).toHaveLength(3);
    for (const it of items) {
      expect(it.textContent).toMatch(/Giống ở: .+/);
      expect(it.textContent).toMatch(/Khác ở: .+/);
      expect(['Số đo', 'Số đo + Hình ảnh']).toContain(within(it).getByTestId('source-badge').textContent);
    }
    expect(container.textContent).not.toMatch(/%|\d\.\d/);
  });

  it('axis keys read as Vietnamese names', async () => {
    render(<SimilarTab api={createMockApi()} detail={await detail()} />);
    expect(screen.getAllByTestId('similar-hit')[0].textContent).not.toMatch(/x_height|avg_width|stem_weight/);
  });

  it('badges follow the source', () => {
    expect(SOURCE_BADGE).toEqual({ axes: 'Số đo', axes_and_image: 'Số đo + Hình ảnh', image: 'Hình ảnh' });
  });

  it('clicking a hit selects that face', async () => {
    const d = await detail();
    render(<SimilarTab api={createMockApi()} detail={d} />);
    const hit = d.similarTop[1];
    act(() => { fireEvent.click(screen.getAllByTestId('similar-hit')[1].querySelector('button')!); });
    expect(useUiStore.getState().selectedId).toBe(faceKey(hit.face));
  });

  it('explains when a font has too few measurements', async () => {
    const d = { ...(await detail()), similarTop: [] };
    render(<SimilarTab api={createMockApi()} detail={d} />);
    expect(screen.getByText(/quá ít số đo/)).toBeTruthy();
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
      expect(screen.getByTestId('image-strip').textContent).toMatch(/Đang tải model \(38%\)\. Kết quả hiện chỉ dựa trên số đo/);
    });

    it('E4–E7: the error with its retry action', async () => {
      await strip({ initial: { model: { state: 'checksum_mismatch' } } }, true);
      const note = screen.getByTestId('image-strip');
      expect(note.textContent).toMatch(/Model tải về bị hỏng/);
      expect(within(note).getByRole('button', { name: 'Tải lại' })).toBeTruthy();
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
