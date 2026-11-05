import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useUiStore } from '../../../store/ui';
import { SEARCH_DEBOUNCE_MS, SearchField } from '../SearchField';
import { TopBar } from '../TopBar';

const initial = useUiStore.getState();
const search = () => screen.getByRole('textbox', { name: 'Tìm font' }) as HTMLInputElement;

describe('SearchField', () => {
  beforeEach(() => { useUiStore.setState(initial, true); vi.useFakeTimers(); });
  afterEach(() => vi.useRealTimers());

  it('debounces typing by 120ms before updating the query', () => {
    expect(SEARCH_DEBOUNCE_MS).toBe(120);
    render(<SearchField />);
    fireEvent.change(search(), { target: { value: 'pho' } });
    act(() => { vi.advanceTimersByTime(119); });
    expect(useUiStore.getState().query).toBe('');
    act(() => { vi.advanceTimersByTime(2); });
    expect(useUiStore.getState().query).toBe('pho');
  });

  it('only the last keystroke in a burst reaches the store', () => {
    render(<SearchField />);
    for (const v of ['r', 'ro', 'rob']) { fireEvent.change(search(), { target: { value: v } }); act(() => { vi.advanceTimersByTime(50); }); }
    act(() => { vi.advanceTimersByTime(200); });
    expect(useUiStore.getState().query).toBe('rob');
  });

  it('⌘K and Ctrl+K focus and select the field from anywhere', () => {
    render(<SearchField />);
    fireEvent.change(search(), { target: { value: 'abc' } });
    fireEvent.keyDown(document.body, { key: 'k', metaKey: true });
    expect(document.activeElement).toBe(search());
    (document.activeElement as HTMLInputElement).blur();
    fireEvent.keyDown(document.body, { key: 'k', ctrlKey: true });
    expect(document.activeElement).toBe(search());
  });

  it('Esc clears immediately while focused, the clear button too', () => {
    render(<SearchField />);
    act(() => useUiStore.getState().setQuery('abc'));
    expect(search().value).toBe('abc'); // đồng bộ từ store
    search().focus();
    fireEvent.keyDown(search(), { key: 'Escape' });
    expect(search().value).toBe('');
    expect(useUiStore.getState().query).toBe('');
    fireEvent.change(search(), { target: { value: 'x' } });
    fireEvent.click(screen.getByRole('button', { name: 'Xoá tìm kiếm' }));
    expect(search().value).toBe('');
    expect(useUiStore.getState().query).toBe('');
  });

  it('shows the clear button only while there is text, otherwise the ⌘K hint', () => {
    render(<SearchField />);
    expect(screen.queryByRole('button', { name: 'Xoá tìm kiếm' })).toBeNull();
    expect(screen.getByText('⌘K')).toBeTruthy();
    fireEvent.change(search(), { target: { value: 'a' } });
    expect(screen.getByRole('button', { name: 'Xoá tìm kiếm' })).toBeTruthy();
    expect(screen.queryByText('⌘K')).toBeNull();
  });
});

describe('TopBar', () => {
  beforeEach(() => useUiStore.setState(initial, true));
  const settle = async () => { for (let i = 0; i < 4; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };

  it('vi-toggle and layout segmented control write to the store', async () => {
    render(<TopBar api={createMockApi()} />);
    await settle();
    const vi_ = screen.getByRole('button', { name: /Ặ vi/ });
    expect(vi_.getAttribute('aria-pressed')).toBe('false');
    fireEvent.click(vi_);
    expect(useUiStore.getState().viOnly).toBe(true);
    expect(vi_.getAttribute('aria-pressed')).toBe('true');
    fireEvent.click(screen.getByRole('button', { name: 'Danh sách' }));
    expect(useUiStore.getState().layout).toBe('list');
    expect(screen.getByRole('button', { name: 'Lưới' }).getAttribute('aria-pressed')).toBe('false');
  });

  it('shows the view title with a count that follows the filters', async () => {
    const api = createMockApi();
    render(<TopBar api={api} />);
    await settle();
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Tất cả');
    const total = Number(screen.getByTestId('view-count').textContent);
    expect(total).toBeGreaterThan(10);
    act(() => useUiStore.getState().setViOnly(true)); await settle();
    expect(Number(screen.getByTestId('view-count').textContent)).toBeLessThan(total);
    act(() => useUiStore.getState().setView('src:2')); await settle();
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Dự án khách');
    act(() => useUiStore.getState().setView('active')); await settle();
    expect(screen.getByRole('heading', { level: 1 }).textContent).toBe('Đang bật');
    expect(screen.queryByTestId('view-count')).toBeNull(); // chưa lọc được → không hiện số giả
  });
});
