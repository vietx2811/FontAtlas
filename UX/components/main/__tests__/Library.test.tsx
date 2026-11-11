import { act, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useScanStore } from '../../../store/scan';
import { PRIVACY_URLS, setUrlOpener } from '../../../privacy';
import { fireEvent, within } from '@testing-library/react';
import { useUiStore } from '../../../store/ui';
import { Library } from '../Library';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const initial = useUiStore.getState();

describe('Library', () => {
  beforeEach(() => useUiStore.setState(initial, true));
  afterEach(() => vi.restoreAllMocks());

  it('sends the store filters to query_faces', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'queryFaces');
    render(<Library api={api} width={1200} height={700} />);
    await settle();
    expect(spy).toHaveBeenLastCalledWith(expect.not.objectContaining({ favoritesOnly: true }));

    act(() => useUiStore.getState().setView('star')); await settle();
    expect(spy).toHaveBeenLastCalledWith(expect.objectContaining({ favoritesOnly: true }));

    act(() => useUiStore.getState().setView('src:2')); await settle();
    expect(spy).toHaveBeenLastCalledWith(expect.objectContaining({ sourceIds: [2] }));

    act(() => { useUiStore.getState().setView('all'); useUiStore.getState().setTag('5'); }); await settle();
    expect(spy).toHaveBeenLastCalledWith(expect.objectContaining({ tagIds: [5] }));

    act(() => { useUiStore.getState().setTag(null); useUiStore.getState().setQuery('roboto'); useUiStore.getState().setViOnly(true); }); await settle();
    expect(spy).toHaveBeenLastCalledWith(expect.objectContaining({ text: 'roboto', vi: 'full' }));
  });

  it('Gần đây queries the viewed faces by key; Đang bật queries the activated ones', async () => {
    const api = createMockApi();
    const [a, b] = (await api.queryFaces({ limit: 2 })).items;
    await api.activationActivate([{ hashHex: b.hashHex, faceIndex: 0 }]);
    const spy = vi.spyOn(api, 'queryFaces');
    render(<Library api={api} width={1200} height={700} />);
    await settle();
    act(() => { useUiStore.getState().select(`${a.hashHex}:0`); useUiStore.getState().setView('recent'); }); await settle();
    expect(spy).toHaveBeenLastCalledWith(expect.objectContaining({ faceKeys: [{ hashHex: a.hashHex, faceIndex: 0 }], sort: 'match' }));
    expect(screen.getAllByRole('gridcell')).toHaveLength(1);
    act(() => useUiStore.getState().setView('active')); await settle();
    expect(spy).toHaveBeenLastCalledWith(expect.objectContaining({ faceKeys: [{ hashHex: b.hashHex, faceIndex: 0 }] }));
    expect(screen.getAllByRole('gridcell')).toHaveLength(1);
  });

  it('selecting a cell writes the face key into the store', async () => {
    const api = createMockApi();
    render(<Library api={api} width={1200} height={700} />);
    await settle();
    act(() => screen.getAllByRole('gridcell')[0].click());
    expect(useUiStore.getState().selectedId).toMatch(/^[0-9a-f]{64}:0$/);
  });

  it('passes slider size and layout to the grid', async () => {
    const api = createMockApi();
    render(<Library api={api} width={1200} height={700} />);
    await settle();
    const cols = () => Number(screen.getByRole('grid').getAttribute('aria-colcount'));
    const before = cols();
    act(() => useUiStore.getState().setSampleSize(72)); await settle();
    expect(cols()).toBeLessThan(before);
    act(() => useUiStore.getState().setLayout('list')); await settle();
    expect(cols()).toBe(1);
  });

  it('marks active faces with the dot and toggles favorites through set_favorite', async () => {
    const api = createMockApi();
    const first = (await api.queryFaces({ limit: 1 })).items[0];
    await api.activationActivate([{ hashHex: first.hashHex, faceIndex: 0 }]);
    const fav = vi.spyOn(api, 'setFavorite');
    render(<Library api={api} width={1200} height={700} />);
    await settle();
    expect(screen.getAllByTestId('active-dot')).toHaveLength(1);
    const star = screen.getAllByRole('button', { name: 'Yêu thích' })[0];
    expect(star.getAttribute('aria-pressed')).toBe('false');
    await act(async () => { star.click(); });
    await settle();
    expect(fav).toHaveBeenCalledWith({ hashHex: first.hashHex, faceIndex: 0 }, true);
    expect(screen.getAllByRole('button', { name: 'Yêu thích' })[0].getAttribute('aria-pressed')).toBe('true');
  });

  it('highlights the cell of a face selected elsewhere (store selectedId)', async () => {
    const api = createMockApi();
    render(<Library api={api} width={1200} height={700} />);
    await settle();
    const cell = screen.getAllByRole('gridcell')[2];
    const row = (await api.queryFaces({ limit: 5 })).items[2];
    act(() => useUiStore.getState().select(`${row.hashHex}:${row.faceIndex}`));
    expect(cell.getAttribute('aria-selected')).toBe('true');
    expect(screen.getAllByRole('gridcell')[0].getAttribute('aria-selected')).toBe('false');
  });

  it('un-starring in the Yêu thích view drops the card right away', async () => {
    const api = createMockApi();
    const [a, b] = (await api.queryFaces({ limit: 2 })).items;
    await api.setFavorite({ hashHex: a.hashHex, faceIndex: 0 }, true);
    await api.setFavorite({ hashHex: b.hashHex, faceIndex: 0 }, true);
    render(<Library api={api} width={1200} height={700} />);
    act(() => useUiStore.getState().setView('star')); await settle();
    expect(screen.getAllByRole('gridcell')).toHaveLength(2);
    await act(async () => { screen.getAllByRole('button', { name: 'Yêu thích' })[0].click(); });
    await settle();
    expect(screen.getAllByRole('gridcell')).toHaveLength(1);
  });

  it('reloads the grid when a scan adds fonts (no stale "0 fonts" after the scan finishes)', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'queryFaces');
    render(<Library api={api} width={1200} height={700} />);
    await settle();
    const before = spy.mock.calls.length;
    act(() => useScanStore.getState().bump());
    await settle();
    expect(spy.mock.calls.length).toBeGreaterThan(before);
    expect(spy.mock.calls.at(-1)![0]).toMatchObject({ offset: 0 });
  });

  describe('zero states', () => {
    const empty = () => ({ total: 0, offset: 0, items: [] });
    const zero = () => screen.getByTestId('zero-state');

    it('no sources: invites to add a folder (asks the sidebar to open the picker)', async () => {
      const api = createMockApi();
      vi.spyOn(api, 'queryFaces').mockResolvedValue(empty());
      vi.spyOn(api, 'listSources').mockResolvedValue([]);
      render(<Library api={api} width={1200} height={700} />);
      await settle();
      expect(zero().textContent).toMatch(/Kéo thư mục/);
      const before = useUiStore.getState().addFolderRequest;
      fireEvent.click(within(zero()).getByRole('button', { name: 'Thêm thư mục font' }));
      expect(useUiStore.getState().addFolderRequest).toBe(before + 1);
    });

    it('a search with no hits names the text; the action clears query, vi filter and tag', async () => {
      const api = createMockApi();
      render(<Library api={api} width={1200} height={700} />);
      await settle();
      act(() => { useUiStore.getState().setQuery('zzzz'); useUiStore.getState().setViOnly(true); useUiStore.getState().setTag('3'); }); await settle();
      expect(zero().textContent).toContain('“zzzz”');
      fireEvent.click(within(zero()).getByRole('button', { name: 'Xoá bộ lọc' }));
      expect(useUiStore.getState()).toMatchObject({ query: '', viOnly: false }); expect(useUiStore.getState().filters.tagIds).toBeUndefined();
    });

    it('favorites, recent and active show their one-liners', async () => {
      const api = createMockApi();
      render(<Library api={api} width={1200} height={700} />);
      await settle();
      for (const [view, re] of [['star', /ngôi sao/], ['recent', /gần đây/], ['active', /công tắc/]] as const) {
        act(() => useUiStore.getState().setView(view)); await settle();
        expect(zero().textContent, view).toMatch(re);
      }
    });

    it('an empty project goes back to the library; an empty tag can be dropped', async () => {
      const api = createMockApi();
      const col = await api.createCollection('Menu', null);
      const tag = await api.createTag('serif');
      render(<Library api={api} width={1200} height={700} />);
      await settle();
      act(() => useUiStore.getState().setView(`col:${col.id}`)); await settle();
      fireEvent.click(within(zero()).getByRole('button', { name: 'Thêm font' }));
      expect(useUiStore.getState().view).toBe('all');
      act(() => useUiStore.getState().setTag(String(tag.id))); await settle();
      expect(zero().textContent).toMatch(/gắn thẻ này/);
    });

    it('an unplugged drive explains itself and offers a rescan of that source', async () => {
      const api = createMockApi();
      const spy = vi.spyOn(api, 'rescanSources');
      vi.spyOn(api, 'queryFaces').mockResolvedValue(empty());
      render(<Library api={api} width={1200} height={700} />);
      await settle();
      act(() => useUiStore.getState().setView('src:3')); await settle();
      expect(zero().textContent).toMatch(/chưa kết nối/);
      fireEvent.click(within(zero()).getByRole('button', { name: 'Quét lại khi cắm' }));
      expect(spy).toHaveBeenCalledWith([3]);
    });

    it('a source that needs access shows Cấp quyền (opens Files and Folders) and Thử lại (rescans just that source)', async () => {
      const api = createMockApi();
      const opened: string[] = [];
      setUrlOpener(async u => { opened.push(u); return true; });
      const spy = vi.spyOn(api, 'rescanSources');
      vi.spyOn(api, 'queryFaces').mockResolvedValue(empty());
      render(<Library api={api} width={1200} height={700} />);
      await settle();
      act(() => { useScanStore.setState({ access: { 3: 'timeout' } }); useUiStore.getState().setView('src:3'); }); await settle();
      expect(zero().textContent).toMatch(/quyền truy cập/);
      fireEvent.click(within(zero()).getByRole('button', { name: 'Cấp quyền' }));
      expect(opened).toEqual([PRIVACY_URLS.files]);
      fireEvent.click(within(zero()).getByRole('button', { name: 'Thử lại' }));
      expect(spy).toHaveBeenCalledWith([3]);
      act(() => useScanStore.setState({ access: {} }));
    });
  });
});