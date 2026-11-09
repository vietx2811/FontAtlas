import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useUiStore } from '../../../store/ui';
import { LangBadge } from '../../../features/library-grid/LangBadge';
import { LangPicker } from '../LangPicker';
import { groupFilters } from '../groupFilters';

const initial = useUiStore.getState();
const settle = async () => { await act(async () => { await new Promise(r => setTimeout(r, 260)); }); };
const filters = () => useUiStore.getState().filters;

describe('LangPicker', () => {
  beforeEach(() => { useUiStore.setState(initial, true); });

  it('lists languages with counts; a row cycles off → full → near → off', async () => {
    render(<LangPicker api={createMockApi()} query={{}} />); await settle();
    const fra = () => document.querySelector('[data-code=fra]') as HTMLElement;
    expect(fra()).not.toBeNull();
    fireEvent.click(fra()); expect(filters().langs).toEqual([{ code: 'fra', level: 'full' }]);
    fireEvent.click(fra()); expect(filters().langs).toEqual([{ code: 'fra', level: 'near' }]);
    fireEvent.click(fra()); expect(filters().langs).toBeUndefined(); expect(filters().langMode).toBeUndefined();
  });

  it('the all/any switch appears with 2+ languages and writes langMode', async () => {
    render(<LangPicker api={createMockApi()} query={{}} />); await settle();
    expect(screen.queryByRole('radiogroup')).toBeNull();
    fireEvent.click(document.querySelector('[data-code=fra]')!); fireEvent.click(document.querySelector('[data-code=pol]')!);
    fireEvent.click(screen.getByLabelText('Một trong các ngôn ngữ'));
    expect(filters().langMode).toBe('any');
    fireEvent.click(screen.getByLabelText('Đủ tất cả'));
    expect(filters().langMode).toBeUndefined();
  });

  it('search narrows the list', async () => {
    render(<LangPicker api={createMockApi()} query={{}} />); await settle();
    fireEvent.change(screen.getByLabelText('Tìm ngôn ngữ'), { target: { value: 'tur' } });
    expect(document.querySelectorAll('.fa-lang-row')).toHaveLength(1);
  });

  it('a selected language with no matching face stays listed so it can be removed', async () => {
    useUiStore.setState({ ...useUiStore.getState(), filters: { langs: [{ code: 'zzz', level: 'full' }] } });
    render(<LangPicker api={createMockApi()} query={{}} />); await settle();
    expect(document.querySelector('[data-code=zzz]')).not.toBeNull();
  });

  it('the mock filter really narrows faces by language', async () => {
    const api = createMockApi();
    const all = (await api.queryFaces({ limit: 1000 })).total;
    const full = (await api.queryFaces({ limit: 1000, langs: [{ code: 'fra', level: 'full' }] })).total;
    const near = (await api.queryFaces({ limit: 1000, langs: [{ code: 'fra', level: 'near' }] })).total;
    expect(full).toBeGreaterThan(0); expect(full).toBeLessThan(all); expect(near).toBeGreaterThanOrEqual(full);
  });
});

describe('language chips and groups', () => {
  it('langs form their own group with removable chips', () => {
    const f = { langs: [{ code: 'vie', level: 'full' as const }, { code: 'fra', level: 'near' as const }], langMode: 'any' as const };
    const g = groupFilters(f);
    expect(g.map(x => x.id)).toEqual(['lang']);
    expect(g[0].chips).toHaveLength(2);
    expect(g[0].chips[0].remove(f)).toEqual({ langs: [{ code: 'fra', level: 'near' }], langMode: 'any' });
    expect(g[0].remove(f)).toEqual({});
  });
});

describe('LangBadge', () => {
  const mk = (state: 'full' | 'near' | 'partial' | 'low' | 'unknown', have = 120, code = 'vie') => <LangBadge badge={{ code, bcp47: code === 'vie' ? 'vi' : 'fr', state, have, total: 134 }} />;
  it.each([['full', '●'], ['near', '◐'], ['partial', '−14'], ['low', '−14'], ['unknown', '?']] as const)('%s → %s', (state, mark) => {
    render(mk(state)); expect(screen.getByTestId('vi-mark').textContent).toBe(mark);
  });
  it('only partial and low carry the warning flag', () => {
    const { unmount } = render(mk('partial')); expect(screen.getByTestId('vi-mark').dataset.warn).toBe('true'); unmount();
    render(mk('near')); expect(screen.getByTestId('vi-mark').dataset.warn).toBeUndefined();
  });
  it('a non-Vietnamese badge shows its language code', () => {
    render(mk('full', 134, 'fra')); expect(screen.getByTestId('vi-mark').querySelector('.fa-vi-code')?.textContent).toBe('fra');
  });
});
