import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useUiStore } from '../../../store/ui';
import { DuplicatesPage } from '../DuplicatesPage';

const initial = useUiStore.getState();
beforeEach(() => useUiStore.setState(initial, true));
const mount = async (api = createMockApi()) => { render(<DuplicatesPage api={api} />); await screen.findAllByTestId('dup-group'); return api; };

describe('DuplicatesPage', () => {
  it('groups by kind with English terms + gloss and counts', async () => {
    await mount();
    for (const term of ['Same file', 'Versions', 'Formats']) expect(screen.getByRole('heading', { name: new RegExp(term) })).toBeTruthy();
    expect(screen.getByText(/cùng tệp, nhiều đường dẫn/)).toBeTruthy();
    expect(screen.getAllByTestId('dup-group').length).toBeGreaterThanOrEqual(4);
  });

  it('each member shows name, path (tooltip has the full one), format, version/revision, date', async () => {
    await mount();
    const g = screen.getAllByTestId('dup-group').find(x => /PlayfairDisplay-Regular/.test(x.textContent ?? ''))!;
    const members = within(g).getAllByTestId('dup-member');
    expect(members).toHaveLength(2);
    expect(within(members[0]).getByText('TTF')).toBeTruthy();
    expect(within(members[0]).getByText(/Version 1\.203/)).toBeTruthy();
    expect(within(members[0]).getByText(/1,203/)).toBeTruthy();
    expect(within(members[0]).getByTestId('dup-path').getAttribute('title')).toMatch(/^\//);
  });

  it('marks only the newest member with "Mới nhất"; an undecidable group says so', async () => {
    await mount();
    expect(screen.getAllByText('Mới nhất')).toHaveLength(1);
    expect(screen.getAllByText('Không xác định được bản mới nhất').length).toBeGreaterThanOrEqual(1);
  });

  it('shows a dash for unknown version, date and revision and a readable chip for unknown formats', async () => {
    await mount();
    const roboto = screen.getAllByTestId('dup-group').find(x => /Roboto-Regular/.test(x.textContent ?? ''))!;
    expect(within(roboto).getAllByText('—').length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText('SFNT')).toBeTruthy();
  });

  it('offers only viewing actions: open detail and (when the path is the face path) reveal in Finder — never delete or move', async () => {
    const api = await mount();
    const reveal = vi.spyOn(api, 'revealInFinder');
    expect(screen.queryByRole('button', { name: /xoá|xóa|delete|di chuyển|move|gộp/i })).toBeNull();
    const first = screen.getAllByTestId('dup-member')[0];
    fireEvent.click(within(first).getByRole('button', { name: 'Hiện trong Finder' }));
    expect(reveal).toHaveBeenCalledTimes(1);
    fireEvent.click(within(first).getByRole('button', { name: 'Mở chi tiết' }));
    expect(useUiStore.getState().selectedId).toBeTruthy();
    // đường dẫn khác đường dẫn của face: không có nút Finder (không chỉ sai chỗ)
    const second = screen.getAllByTestId('dup-member')[1];
    expect(within(second).queryByRole('button', { name: 'Hiện trong Finder' })).toBeNull();
  });

  it('passes the current filters to both calls', async () => {
    const api = createMockApi();
    useUiStore.getState().setFilters({ italic: false });
    useUiStore.getState().setQuery('play');
    const g = vi.spyOn(api, 'duplicateGroups');
    await mount(api);
    expect(g).toHaveBeenLastCalledWith(expect.objectContaining({ italic: false, text: 'play' }));
  });

  it('empty state: a small illustration, one line, no groups', async () => {
    const api = createMockApi();
    vi.spyOn(api, 'duplicateGroups').mockResolvedValue([]);
    render(<DuplicatesPage api={api} />);
    expect(await screen.findByTestId('zero-state')).toHaveTextContent('Không có font trùng');
    expect(screen.queryByTestId('dup-group')).toBeNull();
  });

  it('error state with retry', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'duplicateGroups').mockRejectedValueOnce(new Error('boom'));
    render(<DuplicatesPage api={api} />);
    expect(await screen.findByRole('alert')).toHaveTextContent('boom');
    spy.mockRestore();
    fireEvent.click(screen.getByRole('button', { name: 'Thử lại' }));
    await screen.findAllByTestId('dup-group');
  });

  it('survives unknown group kinds from a newer backend', async () => {
    const api = createMockApi();
    const g = await api.duplicateGroups();
    vi.spyOn(api, 'duplicateGroups').mockResolvedValue([...g, { ...g[0], kind: 'brand_new_kind' as never, key: 'k' }]);
    await mount(api);
    expect(screen.getByRole('heading', { name: /Brand new kind/ })).toBeTruthy();
  });
});
