import { fireEvent, render, screen, waitFor } from '@testing-library/react';
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
