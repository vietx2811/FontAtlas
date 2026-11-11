import { act, fireEvent, render, renderHook, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { SearchField } from '../../main/SearchField';
import { useNoticeStore } from '../../../store/notices';
import { toMetadataQuery, useUiStore } from '../../../store/ui';
import { AdvancedFilters } from '../AdvancedFilters';
import { LangPicker } from '../LangPicker';
import { groupFilters, restoreChanged } from '../groupFilters';
import { COALESCE_MS, rangeOnly, useFilterHistory } from '../useFilterHistory';
import { useAxisHistograms } from '../useAxisHistograms';

const initial = useUiStore.getState();
const set = (f: object) => act(() => { useUiStore.getState().setFilters(f); });
const filters = () => useUiStore.getState().filters;
const settle = async (ms = 260) => { await act(async () => { await new Promise(r => setTimeout(r, ms)); }); };
const input = () => screen.getByLabelText('Tìm font');

describe('review fixes', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });

  it('holding Backspace (repeat) never drops more than the selection step', () => {
    render(<SearchField />);
    set({ classes: ['serif'], license: ['ofl'], italic: true });
    input().focus();
    fireEvent.keyDown(input(), { key: 'Backspace' });
    for (let i = 0; i < 5; i++) fireEvent.keyDown(input(), { key: 'Backspace', repeat: true });
    expect(Object.keys(filters()).sort()).toEqual(['classes', 'italic', 'license']);
    fireEvent.keyDown(input(), { key: 'Backspace' });
    expect(filters().license).toBeUndefined();
  });

  it('the toast Undo restores only what that removal dropped and keeps later edits', () => {
    render(<SearchField />);
    set({ classes: ['serif'], license: ['ofl'] });
    input().focus();
    fireEvent.keyDown(input(), { key: 'Backspace' }); fireEvent.keyDown(input(), { key: 'Backspace' }); // bỏ license
    set({ ...filters(), italic: true }); // thao tác sau đó
    act(() => { useNoticeStore.getState().notices[0].action!.run(); });
    expect(filters()).toEqual({ classes: ['serif'], italic: true, license: ['ofl'] });
  });

  it('restoreChanged is a pure key-wise restore', () => {
    expect(restoreChanged({ license: ['ofl'], italic: true }, { italic: true }, { italic: true, vi: 'full' })).toEqual({ italic: true, vi: 'full', license: ['ofl'] });
  });

  it('history: slider-only edits within 400 ms are one undo step; view change clears the stack', () => {
    const { result } = renderHook(() => useFilterHistory());
    act(() => { useUiStore.getState().setFilters({ license: ['ofl'] }); });
    act(() => { useUiStore.getState().setFilters({ license: ['ofl'], rawAxisRanges: [{ axis: 'stem', range: [0.1, 0.9] }] }); });
    act(() => { useUiStore.getState().setFilters({ license: ['ofl'], rawAxisRanges: [{ axis: 'stem', range: [0.2, 0.9] }] }); });
    act(() => { useUiStore.getState().setFilters({ license: ['ofl'], rawAxisRanges: [{ axis: 'stem', range: [0.3, 0.9] }] }); });
    act(() => { result.current.undo(); });
    expect(filters()).toEqual({ license: ['ofl'] }); // 3 lần kéo → 1 bước (bước đầu giữ trạng thái trước khi kéo)
    act(() => { result.current.undo(); });
    expect(filters()).toEqual({});
    expect(result.current.canUndo).toBe(false);
    act(() => { useUiStore.getState().setFilters({ italic: true }); });
    expect(result.current.canUndo).toBe(true);
    act(() => { useUiStore.getState().setView('star'); });
    expect(result.current.canUndo).toBe(false);
    expect(COALESCE_MS).toBe(400);
    expect(rangeOnly({ italic: true }, { italic: true, weightClass: [400, 700] })).toBe(true);
    expect(rangeOnly({}, { italic: true })).toBe(false);
  });

  it('Esc closes the "Đang lọc" popover first and leaves the Advanced sheet open; the next Esc closes the sheet', async () => {
    const api = createMockApi();
    render(<><SearchField /><AdvancedFilters api={api} /></>);
    set({ italic: true });
    fireEvent.click(screen.getByTestId('advanced-open')); await settle();
    fireEvent.click(screen.getByTestId('filter-summary'));
    expect(screen.getByTestId('active-filters')).not.toBeNull();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByTestId('active-filters')).toBeNull();
    expect(screen.getByTestId('advanced-sheet')).not.toBeNull();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByTestId('advanced-sheet')).toBeNull();
  });

  it('the popover is rendered outside the search label so clicking in it does not pull focus into the input', () => {
    render(<SearchField />);
    set({ italic: true });
    fireEvent.click(screen.getByTestId('filter-summary'));
    expect(screen.getByTestId('active-filters').closest('label')).toBeNull();
  });

  it('the summary shows how several languages combine (một trong / đủ tất cả)', () => {
    const f = { langs: [{ code: 'vie', level: 'full' as const }, { code: 'fra', level: 'full' as const }] };
    expect(groupFilters({ ...f, langMode: 'any' })[0].suffix).toBe('một trong');
    expect(groupFilters(f)[0].suffix).toBe('đủ tất cả');
    expect(groupFilters({ langs: [f.langs[0]] })[0].suffix).toBeUndefined();
    render(<SearchField />); set({ ...f, langMode: 'any' });
    expect(screen.getByTestId('filter-summary').textContent).toMatch(/một trong/);
  });

  it('the Advanced button and the sheet count conditions the same way as the summary (varAxisMatch alone is not a condition)', () => {
    render(<SearchField />);
    set({ varAxisMatch: { wght: false, wdth: true } });
    expect(screen.getByTestId('advanced-open').getAttribute('data-filtered')).toBe('false');
    expect(screen.queryByTestId('advanced-count')).toBeNull();
    set({ weightClass: [600, 900], varAxisMatch: { wght: false, wdth: true } });
    expect(screen.getByTestId('advanced-open').getAttribute('data-filtered')).toBe('true');
    expect(screen.getByTestId('advanced-count').textContent).toBe('1');
  });

  it('LangPicker rows announce their three states', async () => {
    render(<LangPicker api={createMockApi()} query={{}} />); await settle();
    const row = () => document.querySelector('[data-code=fra]') as HTMLElement;
    expect(row().getAttribute('aria-label')).toMatch(/: chưa chọn$/);
    fireEvent.click(row()); expect(row().getAttribute('aria-label')).toMatch(/: đủ$/);
    fireEvent.click(row()); expect(row().getAttribute('aria-label')).toMatch(/: gần đủ$/);
  });

  it('the card badge follows the first language being filtered', () => {
    expect(toMetadataQuery({ view: 'all', query: '', viOnly: false, filters: { langs: [{ code: 'fra', level: 'full' }] } }).badgeLang).toBe('fra');
    expect(toMetadataQuery({ view: 'all', query: '', viOnly: false, filters: {} }).badgeLang).toBeUndefined();
  });

  it('useAxisHistograms stops reporting loading when it gets disabled', async () => {
    const api = createMockApi();
    const { result, rerender } = renderHook(({ on }) => useAxisHistograms(api, {}, on), { initialProps: { on: true } });
    expect(result.current.loading).toBe(true);
    rerender({ on: false }); await settle(10);
    expect(result.current.loading).toBe(false);
  });
});
