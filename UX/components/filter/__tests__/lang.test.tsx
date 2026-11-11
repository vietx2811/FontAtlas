import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useUiStore } from '../../../store/ui';
import { LangBadge } from '../../../features/library-grid/LangBadge';
import { LangPicker, langRank } from '../LangPicker';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
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
  it.each(['full', 'near', 'partial', 'low'] as const)('%s renders a ring and no digits', state => {
    render(mk(state)); const m = screen.getByTestId('vi-mark');
    expect(m.dataset.state).toBe(state); expect(m.textContent).toBe('VI'); expect(m.querySelector('svg')).not.toBeNull();
  });
  it('the four states differ in shape, not only colour', () => {
    const shape = (s: 'full' | 'near' | 'partial' | 'low') => { const { container, unmount } = render(mk(s, s === 'near' ? 125 : 60)); const r = container.querySelector('svg')!.innerHTML.replace(/stroke-dasharray="[^"]*"/, 'D'); unmount(); return r; };
    expect(new Set([shape('full'), shape('near'), shape('low')]).size).toBe(3);
    const arcs = (s: 'near' | 'partial') => { const { container, unmount } = render(mk(s, s === 'near' ? 125 : 60)); const d = container.querySelector('.arc')!.getAttribute('stroke-dasharray'); unmount(); return d; };
    expect(arcs('near')).not.toBe(arcs('partial'));
  });
  it('unknown shows nothing', () => { const { container } = render(mk('unknown')); expect(container.firstChild).toBeNull(); });
  it('only partial and low carry the warning flag', () => {
    const { unmount } = render(mk('partial')); expect(screen.getByTestId('vi-mark').dataset.warn).toBe('true'); unmount();
    render(mk('near')); expect(screen.getByTestId('vi-mark').dataset.warn).toBeUndefined();
  });
  it('shows the language code in capitals', () => {
    render(mk('full', 134, 'fra')); expect(screen.getByTestId('vi-mark').querySelector('.fa-vi-code')?.textContent).toBe('FR');
  });
});

describe('language ordering and badge colour', () => {
  it('langRank: vie first, common Latin in the stated order, then Cyrillic/Greek, then the rest', () => {
    const codes = [{ code: 'zzz', script: 'Latn' }, { code: 'ell', script: 'Grek' }, { code: 'rus', script: 'Cyrl' }, { code: 'ces', script: 'Latn' }, { code: 'eng', script: 'Latn' }, { code: 'vie', script: 'Latn' }, { code: 'fra', script: 'Latn' }];
    expect([...codes].sort((a, b) => langRank(a) - langRank(b)).map(c => c.code)).toEqual(['vie', 'eng', 'fra', 'ces', 'rus', 'ell', 'zzz']);
  });
  it('the picker renders vie before eng before fra', async () => {
    render(<LangPicker api={createMockApi()} query={{}} />); await settle();
    const order = [...document.querySelectorAll('.fa-lang-row')].map(r => r.getAttribute('data-code'));
    const at = (c: string) => order.indexOf(c);
    if (at('vie') >= 0) expect(at('vie')).toBe(0);
    if (at('eng') >= 0 && at('fra') >= 0) expect(at('eng')).toBeLessThan(at('fra'));
  });
  it('the language badge never uses the green "on" token (only the status dot does)', () => {
    const css = readFileSync(join(__dirname, '../../../features/library-grid/libraryGrid.css'), 'utf8');
    const rules = css.split('\n').filter(l => /\.fa-(lang-badge|vi-ring|vi\b|vi\.)/.test(l) && !/\.fa-vi-code/.test(l));
    expect(rules.length).toBeGreaterThan(3);
    for (const r of rules) expect(r).not.toMatch(/var\(--on\)/);
  });
});
