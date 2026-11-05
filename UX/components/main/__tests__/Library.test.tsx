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

  it('shows a pending state, not a grid, for views the backend cannot filter yet', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'queryFaces');
    render(<Library api={api} width={1200} height={700} />);
    await settle();
    spy.mockClear();
    act(() => useUiStore.getState().setView('recent')); await settle();
    expect(screen.getByTestId('library-pending')).toBeTruthy();
    expect(screen.queryByRole('grid')).toBeNull();
    expect(spy).not.toHaveBeenCalled();
  });

  it('selecting a cell writes the face key into the store', async () => {
    const api = createMockApi();
    render(<Library api={api} width={1200} height={700} />);
    await settle();
    act(() => screen.getAllByRole('gridcell')[0].click());
    expect(useUiStore.getState().selectedId).toMatch(/^[0-9a-f]{64}:0$/);
  });
});
