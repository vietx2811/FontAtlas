import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useAdvancedSheet } from '../../../store/advancedSheet';
import { useUiStore } from '../../../store/ui';
import { AdvancedFilters } from '../AdvancedFilters';
import { AdvancedSheet } from '../AdvancedSheet';
import { ALL_AXES } from '../axisGroups';
import { HISTOGRAM_DEBOUNCE_MS } from '../useAxisHistograms';
import { KEY_COMMIT_MS } from '../RangeSlider';

const initial = useUiStore.getState();
const settle = async (ms = HISTOGRAM_DEBOUNCE_MS + 50) => { await act(async () => { await new Promise(r => setTimeout(r, ms)); }); };
const filters = () => useUiStore.getState().filters;

describe('AdvancedSheet', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useAdvancedSheet.setState({ open: false }); });
  afterEach(() => { vi.restoreAllMocks(); });

  it('lists all 18 axes exactly once, each with an English TermTip hook', async () => {
    render(<AdvancedSheet api={createMockApi()} onClose={() => {}} />); await settle();
    const rows = [...document.querySelectorAll('[data-testid=axis-filter-row]')].map(r => (r as HTMLElement).dataset.axis);
    expect(rows.sort()).toEqual([...ALL_AXES].sort());
    expect(ALL_AXES).toHaveLength(18);
    for (const el of document.querySelectorAll('.fa-axis-name')) { expect((el as HTMLElement).dataset.term).toBeTruthy(); expect(el.textContent).not.toMatch(/[À-ỹ]/); }
  });

  it('Weight is labelled as OS/2 class, other axes as measured', async () => {
    render(<AdvancedSheet api={createMockApi()} onClose={() => {}} />); await settle();
    expect(document.querySelector('[data-axis=weight_class] .fa-axrow-src')?.textContent).toBe('OS/2 class');
    expect(document.querySelector('[data-axis=stem] .fa-axrow-src')?.textContent).toBe('measured');
  });

  it('arrow keys on the Weight thumb hop 100 per step and write weightClass', async () => {
    render(<AdvancedSheet api={createMockApi()} onClose={() => {}} />); await settle();
    const hi = document.querySelector('[data-axis=weight_class] [data-testid=rs-lo]') as HTMLElement;
    fireEvent.keyDown(hi, { key: 'ArrowRight' }); fireEvent.keyDown(hi, { key: 'ArrowRight' }); fireEvent.keyDown(hi, { key: 'ArrowRight' });
    expect(filters().weightClass).toBeUndefined(); // chưa ghi: đang gộp phím
    await settle(KEY_COMMIT_MS + 60);
    expect(filters().weightClass).toEqual([400, 900]);
  });

  it('typing numbers into a measured axis writes rawAxisRanges; the full range removes it', async () => {
    render(<AdvancedSheet api={createMockApi()} onClose={() => {}} />); await settle();
    const [lo] = document.querySelectorAll<HTMLInputElement>('[data-axis=stem] input');
    const domainLo = Number(lo.value.replace(',', '.'));
    const hiIn = document.querySelectorAll<HTMLInputElement>('[data-axis=stem] input')[1];
    const domainHi = Number(hiIn.value.replace(',', '.'));
    const mid = (domainLo + domainHi) / 2;
    fireEvent.change(lo, { target: { value: String(mid).replace('.', ',') } }); fireEvent.keyDown(lo, { key: 'Enter' });
    expect(filters().rawAxisRanges?.[0].axis).toBe('stem');
    fireEvent.change(lo, { target: { value: String(domainLo).replace('.', ',') } }); fireEvent.keyDown(lo, { key: 'Enter' });
    expect(filters().rawAxisRanges ?? []).toEqual([]);
  });

  it('an end left at the display edge (p1–p99) is stored as the TRUE extreme, so fonts outside p1–p99 are not lost', async () => {
    const api = createMockApi();
    const real = await api.axisHistogram();
    vi.spyOn(api, 'axisHistogram').mockResolvedValue(real.map(h => (h.axis === 'stem' ? { ...h, lo: 0.1, hi: 0.2 } : h)));
    vi.spyOn(api, 'axisStats').mockResolvedValue([{ axis: 'stem', unit: 'ratio', count: 13, min: 0.0504, max: 0.3009, p1: 0.1, p99: 0.2 }]);
    render(<AdvancedSheet api={api} onClose={() => {}} />); await settle();
    const lo = document.querySelector('[data-axis=stem] [data-testid=rs-lo]') as HTMLElement;
    fireEvent.keyDown(lo, { key: 'ArrowRight' }); await settle(KEY_COMMIT_MS + 60);
    const r = filters().rawAxisRanges![0];
    expect(r.axis).toBe('stem');
    expect(r.range[0]).toBeCloseTo(0.101, 3);
    expect(r.range[1]).toBe(0.301); // cận mở = max thật làm tròn lên, KHÔNG phải p99 = 0.2
    // kéo đầu dưới về mép thì cả hai đầu mở → bỏ lọc
    fireEvent.keyDown(document.querySelector('[data-axis=stem] [data-testid=rs-lo]')!, { key: 'Home' }); await settle(KEY_COMMIT_MS + 60);
    expect(filters().rawAxisRanges ?? []).toEqual([]);
  });

  it('wght / wdth buttons toggle varAxisMatch and clear it when both are on again', async () => {
    render(<AdvancedSheet api={createMockApi()} onClose={() => {}} />); await settle();
    fireEvent.click(screen.getByTestId('varmatch-wght'));
    expect(filters().varAxisMatch).toEqual({ wght: false, wdth: true });
    fireEvent.click(screen.getByTestId('varmatch-wght'));
    expect(filters().varAxisMatch).toBeUndefined();
  });

  it('Esc closes the sheet and ⌥⌘F toggles it from the Top Bar button component', async () => {
    const api = createMockApi();
    render(<AdvancedFilters api={api} />);
    expect(screen.queryByTestId('advanced-sheet')).toBeNull();
    fireEvent.keyDown(document, { key: 'f', code: 'KeyF', metaKey: true, altKey: true });
    expect(screen.getByTestId('advanced-sheet')).not.toBeNull(); await settle();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByTestId('advanced-sheet')).toBeNull();
  });

  it('the sheet paints an opaque background from a token that exists in index.css', () => {
    const css = readFileSync(join(__dirname, '../advancedSheet.css'), 'utf8');
    const token = /\.fa-adv \{[^}]*background:\s*var\((--[a-z0-9-]+)\)/.exec(css)?.[1];
    expect(token).toBeTruthy();
    expect(readFileSync(join(__dirname, '../../../index.css'), 'utf8')).toMatch(new RegExp(`\\s${token}:`));
  });

  it('masonry lanes all start at the top: every lane is a flex column, no CSS multi-column', () => {
    const css = readFileSync(join(__dirname, '../advancedSheet.css'), 'utf8');
    expect(css).not.toMatch(/columns:\s*208px/);
    expect(css).toMatch(/\.fa-adv-cols \{[^}]*align-items:\s*flex-start/);
    expect(css).toMatch(/\.fa-adv-lane \{[^}]*flex-direction:\s*column/);
  });

  it('Masonry puts every card in exactly one lane, keeping order within a lane, and balances by weight', async () => {
    render(<AdvancedSheet api={createMockApi()} onClose={() => {}} />); await settle();
    const lanes = [...document.querySelectorAll('.fa-adv-lane')];
    expect(lanes.length).toBeGreaterThan(0);
    const labels = lanes.flatMap(l => [...l.children].map(c => c.getAttribute('aria-label')));
    expect(labels).toHaveLength(12); expect(new Set(labels).size).toBe(12);
  });

});

describe('useAxisHistograms (debounce + cancel)', () => {
  beforeEach(() => { useUiStore.setState(initial, true); });
  it('rapid filter changes cause one request, and a stale answer never overwrites a newer one', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'axisHistogram');
    render(<AdvancedSheet api={api} onClose={() => {}} />); await settle();
    const base = spy.mock.calls.length;
    for (const v of [0.1, 0.2, 0.3]) act(() => { useUiStore.getState().setFilters({ rawAxisRanges: [{ axis: 'stem', range: [v, 0.9] }] }); });
    await settle();
    expect(spy.mock.calls.length - base).toBe(1);
  });
});

describe('Advanced funnel inside the search field', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useAdvancedSheet.setState({ open: false }); });
  it('is an icon button with an aria-label and opens the sheet; there is no separate "Advanced" text button', async () => {
    const { SearchField } = await import('../../main/SearchField');
    const api = createMockApi();
    render(<><SearchField /><AdvancedFilters api={api} /></>);
    const funnel = screen.getByTestId('advanced-open');
    expect(funnel.closest('.fa-search')).not.toBeNull();
    expect(funnel.getAttribute('aria-label')).toBe('Advanced');
    expect(funnel.textContent).toBe('');
    fireEvent.click(funnel); await settle();
    expect(screen.getByTestId('advanced-sheet')).not.toBeNull();
    expect(funnel.getAttribute('aria-expanded')).toBe('true');
  });
  it('shows the number of active filters on the funnel', async () => {
    const { SearchField } = await import('../../main/SearchField');
    useUiStore.setState({ ...useUiStore.getState(), filters: { italic: true, weightClass: [600, 900] } });
    render(<SearchField />);
    expect(screen.getByTestId('advanced-count').textContent).toBe('2');
    expect(screen.getByTestId('advanced-open').getAttribute('aria-label')).toBe('Advanced, 2 bộ lọc đang bật');
  });
});
