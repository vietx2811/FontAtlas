import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import type { AxisStatsDto } from '../../../api';
import { toGridQuery, useUiStore } from '../../../store/ui';
import { FilterPanel } from '../FilterPanel';
const initial = useUiStore.getState();
const axis: AxisStatsDto = { axis: 'slant_deg', unit: 'deg', count: 10, min: -30, max: 45, p1: -10, p99: 15 };
async function mount(stats = [axis]) {
  const api = createMockApi();
  const spy = vi.spyOn(api, 'axisStats').mockResolvedValue(stats);
  render(<FilterPanel api={api} />);
  fireEvent.click(screen.getByRole('button', { name: 'Lọc' }));
  await screen.findByRole('checkbox', { name: 'Độ nghiêng' });
  return { api, spy };
}
beforeEach(() => useUiStore.setState(initial, true));
describe('FilterPanel', () => {
  it('uses stats focus domain, accepts a min/max outlier and sends exact raw values', async () => {
    const { spy } = await mount();
    fireEvent.click(screen.getByRole('checkbox', { name: 'Độ nghiêng' }));
    const low = screen.getByRole('slider', { name: 'Độ nghiêng cận dưới' });
    expect(low).toHaveAttribute('min', '-10'); expect(low).toHaveAttribute('max', '15');
    fireEvent.click(screen.getByRole('button', { name: 'Toàn khoảng' }));
    expect(low).toHaveAttribute('min', '-30'); expect(low).toHaveAttribute('max', '45');
    const from = screen.getByRole('textbox', { name: 'Độ nghiêng từ' });
    fireEvent.change(from, { target: { value: '-25,5' } }); fireEvent.blur(from);
    expect(toGridQuery(useUiStore.getState()).rawAxisRanges).toEqual([{ axis: 'slant_deg', range: [-25.5, 45] }]);
    expect(screen.getByRole('button', { name: 'Lọc · 1' })).toBeTruthy();
    expect(spy).toHaveBeenCalledTimes(1);
    fireEvent.change(from, { target: { value: '-99' } }); fireEvent.blur(from);
    expect(screen.getByRole('alert')).toHaveTextContent('khoảng hợp lệ');
    expect(toGridQuery(useUiStore.getState()).rawAxisRanges![0].range[0]).toBe(-25.5);
    fireEvent.click(screen.getByRole('button', { name: 'Đặt lại' }));
    expect(toGridQuery(useUiStore.getState()).rawAxisRanges).toBeUndefined();
  });
  it('rounds displayed full boundaries outward so raw endpoint faces are never lost', async () => {
    await mount([{ ...axis, axis: 'slant_deg', min: -10.04, max: 15.04, p1: -5.04, p99: 10.04 }]);
    fireEvent.click(screen.getByRole('checkbox', { name: 'Độ nghiêng' }));
    fireEvent.click(screen.getByRole('button', { name: 'Toàn khoảng' }));
    const range = toGridQuery(useUiStore.getState()).rawAxisRanges![0].range;
    expect(range).toEqual([-10.1, 15.1]);
    expect(range[0]).toBeLessThanOrEqual(-10.04); expect(range[1]).toBeGreaterThanOrEqual(15.04);
    expect(screen.getByRole('textbox', { name: 'Độ nghiêng từ' })).toHaveValue('-10,1');
    expect(screen.getByRole('textbox', { name: 'Độ nghiêng đến' })).toHaveValue('15,1');
  });
  it('selects focus floor/ceil and rounds typed lower down, upper up', async () => {
    await mount([{ ...axis, p1: -5.04, p99: 10.04 }]);
    fireEvent.click(screen.getByRole('checkbox', { name: 'Độ nghiêng' }));
    expect(toGridQuery(useUiStore.getState()).rawAxisRanges![0].range).toEqual([-5.1, 10.1]);
    const from = screen.getByRole('textbox', { name: 'Độ nghiêng từ' });
    const to = screen.getByRole('textbox', { name: 'Độ nghiêng đến' });
    fireEvent.change(from, { target: { value: '-3.011' } }); fireEvent.blur(from);
    fireEvent.change(to, { target: { value: '9,011' } }); fireEvent.blur(to);
    expect(toGridQuery(useUiStore.getState()).rawAxisRanges![0].range).toEqual([-3.1, 9.1]);
    expect(from).toHaveValue('-3,1'); expect(to).toHaveValue('9,1');
  });

  it('does not lose a decimal unit from binary floating-point rounding at typed boundaries', async () => {
    await mount([axis, { axis: 'contrast', unit: 'ratio', count: 10, min: 0, max: 2, p1: .5, p99: 1.5 }]);
    fireEvent.click(screen.getByRole('checkbox', { name: 'Tương phản nét' }));
    const from = screen.getByRole('textbox', { name: 'Tương phản nét từ' });
    fireEvent.change(from, { target: { value: '1,001' } }); fireEvent.blur(from);
    expect(toGridQuery(useUiStore.getState()).rawAxisRanges![0].range[0]).toBe(1.001);
    expect(from).toHaveValue('1,001');
  });

  it('only commits slider gestures on release and exposes the unit to assistive technology', async () => {
    await mount(); fireEvent.click(screen.getByRole('checkbox', { name: 'Độ nghiêng' }));
    const changes: unknown[] = [];
    const off = useUiStore.subscribe((s, old) => { if (s.filters !== old.filters) changes.push(s.filters); });
    const low = screen.getByRole('slider', { name: 'Độ nghiêng cận dưới' });
    fireEvent.change(low, { target: { value: '-9.04' } });
    fireEvent.change(low, { target: { value: '-8.04' } });
    expect(changes).toHaveLength(0);
    expect(low).toHaveAttribute('aria-valuetext', '-8,1 °');
    fireEvent.pointerUp(low);
    expect(changes).toHaveLength(1);
    expect(toGridQuery(useUiStore.getState()).rawAxisRanges![0].range).toEqual([-8.1, 15]);
    const high = screen.getByRole('slider', { name: 'Độ nghiêng cận trên' });
    fireEvent.change(high, { target: { value: '14.04' } }); fireEvent.keyUp(high, { key: 'ArrowLeft' });
    expect(changes).toHaveLength(2);
    expect(toGridQuery(useUiStore.getState()).rawAxisRanges![0].range).toEqual([-8.1, 14.1]);
    off();
  });

  it('keeps selected bounds outside a contracted domain and does not commit an unchanged draft', async () => {
    const { spy } = await mount();
    fireEvent.click(screen.getByRole('checkbox', { name: 'Độ nghiêng' }));
    fireEvent.click(screen.getByRole('button', { name: 'Toàn khoảng' }));
    spy.mockResolvedValue([{ ...axis, min: -5, max: 5, p1: -2, p99: 2 }]);
    const from = screen.getByRole('textbox', { name: 'Độ nghiêng từ' });
    act(() => useUiStore.getState().setView('src:2'));
    await waitFor(() => expect(screen.getByText('Min -5,0 · max 5,0')).toBeTruthy());
    expect(screen.getByRole('textbox', { name: 'Độ nghiêng từ' })).toBe(from);
    fireEvent.blur(from);
    expect(screen.queryByRole('alert')).toBeNull();
    expect(toGridQuery(useUiStore.getState()).rawAxisRanges![0].range).toEqual([-30, 45]);
    expect(screen.getByRole('slider', { name: 'Độ nghiêng cận dưới' })).toHaveAttribute('min', '-30');
    expect(screen.getByRole('slider', { name: 'Độ nghiêng cận trên' })).toHaveAttribute('max', '45');
  });

  it('retains drafts and full-domain mode through refetch and ignores recent membership', async () => {
    const { spy } = await mount();
    fireEvent.click(screen.getByRole('checkbox', { name: 'Độ nghiêng' }));
    fireEvent.click(screen.getByRole('button', { name: 'Toàn khoảng' }));
    const from = screen.getByRole('textbox', { name: 'Độ nghiêng từ' });
    fireEvent.change(from, { target: { value: '-24,04' } });
    let resolve!: (s: AxisStatsDto[]) => void;
    spy.mockImplementationOnce(() => new Promise(r => { resolve = r; }));
    act(() => useUiStore.getState().setView('src:2'));
    await screen.findByText('Đang cập nhật miền số đo…');
    expect(screen.getByRole('textbox', { name: 'Độ nghiêng từ' })).toBe(from);
    expect(from).toHaveValue('-24,04');
    await act(async () => resolve([{ ...axis, min: -100, max: 100, p1: -50, p99: 60 }]));
    expect(from).toHaveValue('-24,04');
    expect(screen.getByRole('button', { name: 'Khoảng P1–P99' })).toBeTruthy();
    act(() => useUiStore.getState().setView('recent'));
    await waitFor(() => expect(spy).toHaveBeenLastCalledWith({}));
    const before = spy.mock.calls.length;
    act(() => useUiStore.getState().select('new:0'));
    await act(async () => {});
    expect(spy).toHaveBeenCalledTimes(before);
    expect(screen.getByRole('button', { name: /Lọc ·/ })).not.toHaveAttribute('aria-pressed');
  });

  it('handles missing and constant stats without fabricating a domain', async () => {
    await mount([axis, { axis: 'contrast', unit: 'ratio', count: 1, min: 1, max: 1, p1: 1, p99: 1 }, { axis: 'spacing', unit: 'x_height_ratio', count: 0, min: null, max: null, p1: null, p99: null }]);
    expect(screen.getByRole('checkbox', { name: 'Khoảng cách chữ' })).toBeDisabled();
    fireEvent.click(screen.getByRole('checkbox', { name: 'Tương phản nét' }));
    expect(screen.getByRole('slider', { name: 'Tương phản nét cận dưới' })).toBeDisabled();
    expect(toGridQuery(useUiStore.getState()).rawAxisRanges).toEqual([{ axis: 'contrast', range: [1, 1] }]);
    expect(screen.getByText(/Cùng giá trị 1/)).toBeTruthy();
  });
  it('passes metadata scope, retries errors and closes with Escape restoring focus', async () => {
    const api = createMockApi();
    useUiStore.getState().setView('src:2'); useUiStore.getState().setViOnly(true);
    const spy = vi.spyOn(api, 'axisStats').mockRejectedValueOnce(new Error('offline')).mockResolvedValue([axis]);
    render(<FilterPanel api={api} />);
    const trigger = screen.getByRole('button', { name: 'Lọc' }); fireEvent.click(trigger);
    expect(await screen.findByRole('alert')).toHaveTextContent('offline');
    fireEvent.click(screen.getByRole('button', { name: 'Thử lại' })); await screen.findByRole('checkbox', { name: 'Độ nghiêng' });
    expect(spy).toHaveBeenLastCalledWith({ sourceIds: [2], vi: 'full' });
    fireEvent.click(screen.getByRole('checkbox', { name: 'Variable' }));
    await waitFor(() => expect(spy).toHaveBeenLastCalledWith({ sourceIds: [2], vi: 'full', variable: true }));
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull(); expect(document.activeElement).toBe(trigger);
  });
  it('family scope clears on sidebar or search while filters persist', () => {
    const s = useUiStore.getState(); s.setFilters({ italic: true });
    const family = { name: 'Family thật', faces: [{ hashHex: 'abc', faceIndex: 2 }] }; s.openFamily(family);
    expect(toGridQuery(useUiStore.getState())).toEqual({ italic: true, faceKeys: family.faces });
    s.setQuery('khác'); expect(useUiStore.getState().familyScope).toBeNull();
    expect(toGridQuery(useUiStore.getState())).toEqual({ italic: true, text: 'khác' });
    s.openFamily(family); s.setView('star');
    expect(toGridQuery(useUiStore.getState())).toEqual({ italic: true, favoritesOnly: true });
  });
});
