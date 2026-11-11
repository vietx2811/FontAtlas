import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { toGridQuery, useUiStore } from '../../../store/ui';
import { AdvancedSheet } from '../../filter/AdvancedSheet';
import { filterChips } from '../FilterChips';
import { useSourcesStore } from '../../../store/sources';
import { LICENSE_OPTIONS } from '../filterOptions';

const initial = useUiStore.getState();
beforeEach(() => useUiStore.setState(initial, true));

const mount = async () => {
  const api = createMockApi();
  await useSourcesStore.getState().load(api, `t${Math.random()}`);
  render(<AdvancedSheet api={api} onClose={() => {}} />);
  await screen.findByRole('dialog', { name: 'Advanced filters' });
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
    const drop = (id: string) => { const f = useUiStore.getState().filters; useUiStore.getState().setFilters(filterChips(f).find(c => c.id === id)!.remove(f)); };
    drop('license:ofl');
    expect(useUiStore.getState().filters).toEqual({ vi: 'full', variable: true });
    drop('variable');
    expect(useUiStore.getState().filters).toEqual({ vi: 'full' });
  });

  it('axis ranges appear as chips with real values', () => {
    useUiStore.getState().setFilters({ rawAxisRanges: [{ axis: 'slant_deg', range: [-12, 0] }] });
    const chips = filterChips(useUiStore.getState().filters);
    expect(chips[0].id).toBe('axis:slant_deg');
    expect(chips[0].label).toMatch(/slant I\/T.*[-−]12.*0/);
  });

  it('license keys are the backend ones (ofl | apache | other | unknown)', () => {
    expect(LICENSE_OPTIONS.map(o => o[0])).toEqual(['ofl', 'apache', 'other', 'unknown']);
  });

  it('license other and apache reach the query and the chips', async () => {
    await mount();
    fireEvent.click(screen.getByRole('checkbox', { name: 'Khác / thương mại' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Apache' }));
    expect(toGridQuery(useUiStore.getState()).license).toEqual(['other', 'apache']);
    expect(filterChips(useUiStore.getState().filters).map(c => c.label)).toEqual(['Khác / thương mại', 'Apache']);
  });

  it('the "Ặ vi" toggle and the vi facet are one state: supported is never overwritten by full', () => {
    useUiStore.getState().setFilters({ vi: 'supported' });
    expect(useUiStore.getState().viOnly).toBe(false);
    expect(toGridQuery(useUiStore.getState()).vi).toBe('supported');
    useUiStore.getState().setViOnly(true);
    expect(useUiStore.getState().filters.vi).toBe('full');
    expect(toGridQuery(useUiStore.getState()).vi).toBe('full');
    expect(filterChips(useUiStore.getState().filters).map(c => c.id)).toEqual(['vi']);
    useUiStore.getState().setViOnly(false);
    expect(useUiStore.getState().filters).toEqual({});
  });

  it('a view that already forces the source/favorites hides those chips and disables the facet', async () => {
    useUiStore.getState().setFilters({ sourceIds: [1], favoritesOnly: true });
    useUiStore.getState().setView('src:2');
    expect(toGridQuery(useUiStore.getState()).sourceIds).toEqual([2]);
    expect(filterChips(useUiStore.getState().filters, [], 'src:2').map(c => c.id)).toEqual(['favorites']);
    useUiStore.getState().setView('star');
    expect(filterChips(useUiStore.getState().filters, [], 'star').map(c => c.id)).toEqual(['source:1']);
    await mount();
    expect((screen.getByRole('checkbox', { name: 'Yêu thích' }) as HTMLInputElement).disabled).toBe(true);
  });

  it('the "Bỏ" button clears only the Vietnamese facet', async () => {
    useUiStore.getState().setFilters({ vi: 'supported', variable: true });
    await mount();
    fireEvent.click(screen.getByRole('button', { name: 'Bỏ' }));
    expect(useUiStore.getState().filters).toEqual({ variable: true });
  });

  it('Italic keeps its English term with a Vietnamese gloss', async () => {
    await mount();
    expect(screen.getByRole('checkbox', { name: 'Italic' }).closest('label')!.getAttribute('title')).toBe('chữ nghiêng');
  });

  it('Phân loại group lists classFacets with counts, Display is a class, subclasses after picking a class', async () => {
    await mount();
    const grp = await screen.findByRole('group', { name: 'Phân loại' });
    expect(await within(grp).findByRole('checkbox', { name: /^Serif/ })).toBeTruthy();
    expect(within(grp).getByRole('checkbox', { name: /^Display/ })).toBeTruthy();
    expect(within(grp).queryByRole('checkbox', { name: /Didone/ })).toBeNull();
    fireEvent.click(within(grp).getByRole('checkbox', { name: /^Serif/ }));
    expect(toGridQuery(useUiStore.getState()).classes).toEqual(['serif']);
    fireEvent.click(await within(grp).findByRole('checkbox', { name: /Didone/ }));
    expect(toGridQuery(useUiStore.getState()).subclasses).toEqual(['didone']);
    expect(filterChips(useUiStore.getState().filters).map(c => c.label)).toEqual(['Serif', 'Didone']);
  });

  it('class and subclass chips each remove only themselves', () => {
    useUiStore.getState().setFilters({ classes: ['serif'], subclasses: ['didone'] });
    const drop = (id: string) => { const f = useUiStore.getState().filters; useUiStore.getState().setFilters(filterChips(f).find(c => c.id === id)!.remove(f)); };
    drop('subclass:didone');
    expect(useUiStore.getState().filters).toEqual({ classes: ['serif'] });
    drop('class:serif');
    expect(useUiStore.getState().filters).toEqual({});
  });

  it('Uncertain and Unclassified are separate buckets with their own labels', () => {
    useUiStore.getState().setFilters({ classes: ['uncertain', 'unclassified'] });
    expect(filterChips(useUiStore.getState().filters).map(c => c.label)).toEqual(['Uncertain', 'Chưa phân loại']);
  });

  it('unclassified is its own bucket, separate from uncertain, and the totals add up to the library', async () => {
    const api = createMockApi();
    const facets = await api.classFacets({});
    const un = facets.find(f => f.class === 'unclassified')!;
    expect(un.count).toBeGreaterThan(0);
    expect(facets.find(f => f.class === 'uncertain')!.count).toBeGreaterThan(0);
    expect((await api.queryFaces({ classes: ['unclassified'] })).total).toBe(un.count);
    await mount();
    const grp = await screen.findByRole('group', { name: 'Phân loại' });
    fireEvent.click(await within(grp).findByRole('checkbox', { name: /^Chưa phân loại/ }));
    expect(toGridQuery(useUiStore.getState()).classes).toEqual(['unclassified']);
  });

  it('a selected class stays listed even when facets fail or its count is 0', async () => {
    useUiStore.getState().setFilters({ classes: ['blackletter'] });
    const api = createMockApi();
    vi.spyOn(api, 'classFacets').mockRejectedValue(new Error('down'));
    render(<AdvancedSheet api={api} onClose={() => {}} />);
    const grp = await screen.findByRole('group', { name: 'Phân loại' });
    expect((within(grp).getByRole('checkbox', { name: /Blackletter/ }) as HTMLInputElement).checked).toBe(true);
  });
});
