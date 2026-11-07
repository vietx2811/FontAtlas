import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { toGridQuery, useUiStore } from '../../../store/ui';
import { FilterPanel } from '../FilterPanel';
import { FilterChips, filterChips } from '../FilterChips';

const initial = useUiStore.getState();
beforeEach(() => useUiStore.setState(initial, true));

const mount = async () => {
  const api = createMockApi();
  render(<><FilterPanel api={api} /><FilterChips api={api} /></>);
  fireEvent.click(screen.getByRole('button', { name: /^Lọc/ }));
  await screen.findByRole('dialog', { name: 'Bộ lọc font' });
  return api;
};

describe('filter facets', () => {
  it('Vietnamese, license, favorites and source go into the query', async () => {
    await mount();
    fireEvent.click(screen.getByRole('radio', { name: 'Đủ dấu' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'OFL' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Yêu thích' }));
    await waitFor(() => screen.getByRole('checkbox', { name: /Fonts/ }));
    fireEvent.click(screen.getByRole('checkbox', { name: /Fonts/ }));
    const q = toGridQuery(useUiStore.getState());
    expect(q.vi).toBe('full');
    expect(q.license).toEqual(['ofl']);
    expect(q.favoritesOnly).toBe(true);
    expect(q.sourceIds?.length).toBe(1);
  });

  it('chips mirror active filters and each removes only itself', async () => {
    useUiStore.getState().setFilters({ vi: 'full', license: ['ofl'], variable: true });
    expect(filterChips(useUiStore.getState().filters).map(c => c.id)).toEqual(['vi', 'license:ofl', 'variable']);
    render(<FilterChips api={createMockApi()} />);
    const chips = screen.getByRole('list', { name: 'Bộ lọc đang bật' });
    fireEvent.click(within(chips).getByRole('button', { name: 'Bỏ lọc OFL' }));
    expect(useUiStore.getState().filters).toEqual({ vi: 'full', variable: true });
    fireEvent.click(within(chips).getByRole('button', { name: 'Bỏ lọc Variable' }));
    expect(useUiStore.getState().filters).toEqual({ vi: 'full' });
  });

  it('shows no chip row when nothing is filtered', () => {
    render(<FilterChips api={createMockApi()} />);
    expect(screen.queryByRole('list', { name: 'Bộ lọc đang bật' })).toBeNull();
  });

  it('axis ranges appear as chips with real values', () => {
    useUiStore.getState().setFilters({ rawAxisRanges: [{ axis: 'slant_deg', range: [-12, 0] }] });
    const chips = filterChips(useUiStore.getState().filters);
    expect(chips[0].id).toBe('axis:slant_deg');
    expect(chips[0].label).toMatch(/Độ nghiêng.*[-−]12.*0/);
  });
});
