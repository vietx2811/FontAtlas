import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { SearchField } from '../../main/SearchField';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';

const initial = useUiStore.getState();
const set = (f: object) => act(() => { useUiStore.getState().setFilters(f); });
const filters = () => useUiStore.getState().filters;
const input = () => screen.getByLabelText('Tìm font');

describe('SearchField filter summary', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });

  it('shows nothing without filters; one segment per group with +N overflow', () => {
    render(<SearchField />);
    expect(screen.queryByTestId('filter-summary')).toBeNull();
    set({ classes: ['serif', 'sans'], italic: true, license: ['ofl'] });
    expect(screen.getByTestId('filter-summary').querySelectorAll('.fa-fsum-seg')).toHaveLength(3);
    expect(screen.getByTestId('filter-summary-more').textContent).toBe('+1');
  });

  it('Backspace on empty input selects the last group, the second drops it with an Undo toast; ⌘Z (focus away) brings it back', () => {
    render(<SearchField />);
    set({ classes: ['serif'], license: ['ofl'] });
    input().focus();
    fireEvent.keyDown(input(), { key: 'Backspace' });
    expect(document.querySelector('[data-selected=true]')?.getAttribute('data-group')).toBe('license');
    expect(filters().license).toEqual(['ofl']);
    fireEvent.keyDown(input(), { key: 'Backspace' });
    expect(filters()).toEqual({ classes: ['serif'] });
    const n = useNoticeStore.getState().notices[0];
    expect(n.action?.label).toBe('Hoàn tác');
    act(() => { n.action!.run(); });
    expect(filters()).toEqual({ classes: ['serif'], license: ['ofl'] });
  });

  it('Backspace does nothing to filters while the input has text; typing clears the selection', () => {
    render(<SearchField />);
    set({ italic: true });
    fireEvent.change(input(), { target: { value: 'ab' } });
    fireEvent.keyDown(input(), { key: 'Backspace' });
    expect(filters()).toEqual({ italic: true });
    fireEvent.change(input(), { target: { value: '' } });
    fireEvent.keyDown(input(), { key: 'Backspace' });
    expect(document.querySelector('[data-selected=true]')).not.toBeNull();
    fireEvent.keyDown(input(), { key: 'a' });
    expect(document.querySelector('[data-selected=true]')).toBeNull();
  });

  it('⌘⌫ clears all filters with a toast', () => {
    render(<SearchField />);
    set({ classes: ['serif'], italic: true });
    fireEvent.keyDown(input(), { key: 'Backspace', metaKey: true });
    expect(filters()).toEqual({});
    expect(useNoticeStore.getState().notices[0].text).toBe('Đã xoá tất cả bộ lọc');
  });

  it('popover: × removes one value, Bỏ nhóm removes the group, Esc closes', () => {
    render(<SearchField />);
    set({ classes: ['serif', 'sans'], license: ['ofl'] });
    fireEvent.click(screen.getByTestId('filter-summary'));
    const pop = screen.getByTestId('active-filters');
    expect(pop.querySelectorAll('.fa-fpop-row')).toHaveLength(2);
    fireEvent.click(screen.getAllByRole('button', { name: /^Bỏ / }).find(b => b.textContent === '×')!);
    expect(filters().classes).toHaveLength(1);
    fireEvent.click(pop.querySelector('[data-group=license] .fa-fpop-drop')!);
    expect(filters().license).toBeUndefined();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByTestId('active-filters')).toBeNull();
  });

  it('popover closes by itself when the last filter goes', () => {
    render(<SearchField />);
    set({ italic: true });
    fireEvent.click(screen.getByTestId('filter-summary'));
    fireEvent.click(screen.getByText('Xoá tất cả'));
    expect(screen.queryByTestId('active-filters')).toBeNull();
    expect(filters()).toEqual({});
  });
});
