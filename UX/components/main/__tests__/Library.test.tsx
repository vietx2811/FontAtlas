import { act, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
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
});
