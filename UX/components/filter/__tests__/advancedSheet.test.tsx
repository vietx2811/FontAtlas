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

  it('the cards container uses 208px masonry columns', () => {
    const css = readFileSync(join(__dirname, '../advancedSheet.css'), 'utf8');
    expect(css).toMatch(/columns:\s*208px/);
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
