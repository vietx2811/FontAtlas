import { fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi, mockRows } from '../../../api/mock';
import { useUiStore } from '../../../store/ui';
import { ComparePage } from '../ComparePage';
import { loadRowConfig, ROW_CONFIG_KEY, saveRowConfig } from '../rowConfig';

const initial = useUiStore.getState();
const rows = mockRows();
const faces = rows.slice(0, 3).map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex }));
beforeEach(() => { useUiStore.setState(initial, true); localStorage.clear(); useUiStore.getState().openCompare(faces); });
afterEach(() => vi.restoreAllMocks());
const mount = async () => { render(<ComparePage api={createMockApi()} faces={faces} />); await screen.findByRole('table'); };
const rowIds = () => [...document.querySelectorAll('[data-row]')].map(e => e.getAttribute('data-row'));

describe('H5 · groups and rows', () => {
  it('the − button collapses a group, + opens it again; other groups stay', async () => {
    await mount();
    expect(rowIds()).toContain('metrics.x_height_em');
    fireEvent.click(screen.getByRole('button', { name: /Thu gọn nhóm · Số đo/ }));
    expect(rowIds().some(i => i!.startsWith('metrics.'))).toBe(false);
    expect(rowIds()).toContain('specimen.line');
    fireEvent.click(screen.getByRole('button', { name: /Mở nhóm · Số đo/ }));
    expect(rowIds()).toContain('metrics.x_height_em');
  });
  it('× hides one row; the group + menu lists hidden rows and brings one back', async () => {
    await mount();
    fireEvent.click(screen.getByRole('button', { name: /Ẩn hàng này · x-height/ }));
    expect(rowIds()).not.toContain('metrics.x_height_em');
    fireEvent.click(screen.getByRole('button', { name: /Thêm hàng · Số đo/ }));
    const menu = await screen.findByRole('menu', { name: 'Hàng đang ẩn' });
    fireEvent.click(within(menu).getByRole('menuitem', { name: /x-height/ }));
    expect(rowIds()).toContain('metrics.x_height_em');
    expect(screen.queryByRole('button', { name: /Thêm hàng · Số đo/ })).toBeNull();
  });
  it('is saved in localStorage and restored on the next mount', async () => {
    await mount();
    fireEvent.click(screen.getByRole('button', { name: /Ẩn hàng này · x-height/ }));
    fireEvent.click(screen.getByRole('button', { name: /Thu gọn nhóm · Meta/ }));
    expect(JSON.parse(localStorage.getItem(ROW_CONFIG_KEY)!)).toMatchObject({ collapsed: ['meta'], hidden: ['metrics.x_height_em'] });
    document.body.innerHTML = '';
    await mount();
    expect(rowIds()).not.toContain('metrics.x_height_em');
    expect(rowIds().some(i => i!.startsWith('meta.'))).toBe(false);
  });
});

describe('row config storage is guarded', () => {
  it('survives a broken value, a throwing getItem and a throwing setItem', () => {
    localStorage.setItem(ROW_CONFIG_KEY, '{nope');
    expect(loadRowConfig()).toEqual({ collapsed: [], hidden: [] });
    localStorage.setItem(ROW_CONFIG_KEY, JSON.stringify({ collapsed: ['meta', 'bogus', 3], hidden: ['a', 7] }));
    expect(loadRowConfig()).toEqual({ collapsed: ['meta'], hidden: ['a'] });
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('blocked'); });
    expect(loadRowConfig()).toEqual({ collapsed: [], hidden: [] });
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('quota'); });
    expect(() => saveRowConfig({ collapsed: [], hidden: [] })).not.toThrow();
  });
});
