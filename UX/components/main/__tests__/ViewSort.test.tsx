import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { DEFAULT_SORT, parseSort, serializeSort, SORT_KEY } from '../../../store/sortPrefs';
import { toGridQuery, useUiStore } from '../../../store/ui';
import { ViewOptions } from '../ViewOptions';

const initial = useUiStore.getState();
const q = () => { const s = useUiStore.getState(); return toGridQuery({ view: s.view, query: s.query, viOnly: s.viOnly, recentViews: s.recentViews, active: s.active, filters: s.filters, familyScope: s.familyScope, sort: s.sort }); };
const settle = async (ms = 0) => { await act(async () => { await new Promise(r => setTimeout(r, ms)); }); };

describe('sort prefs', () => {
  it('group flag: parsed, serialized only when on, kept even when the field is unknown', () => {
    expect(parseSort('{"v":1,"by":"added","desc":true,"group":true}')).toEqual({ by: 'added', desc: true, group: true });
    expect(parseSort('{"v":1,"by":"ghost","group":true}')).toEqual({ ...DEFAULT_SORT, group: true });
    expect(parseSort('{"v":1,"by":"added","group":"yes"}')).toEqual({ by: 'added', desc: true });
    expect(JSON.parse(serializeSort({ by: 'weight', desc: false, group: true }))).toEqual({ v: 1, by: 'weight', desc: false, group: true });
    expect(JSON.parse(serializeSort({ by: 'weight', desc: false }))).not.toHaveProperty('group');
  });
  it('parse: valid, unknown field, garbage, match never restored', () => {
    expect(parseSort('{"v":1,"by":"added","desc":true}')).toEqual({ by: 'added', desc: true });
    expect(parseSort('{"v":1,"by":"weight"}')).toEqual({ by: 'weight', desc: false });
    expect(parseSort('{"v":1,"by":"released"}')).toEqual({ by: 'released', desc: true });
    expect(parseSort('{"v":1,"by":"file_created","desc":false}')).toEqual({ by: 'file_created', desc: false });
    for (const bad of [null, 'x', '{"v":1,"by":"ghost"}', '{"v":2,"by":"added"}', '{"v":1,"by":"match","desc":true}']) expect(parseSort(bad as string | null)).toEqual(DEFAULT_SORT);
    expect(JSON.parse(serializeSort({ by: 'match', desc: true }))).toEqual({ v: 1, by: 'family', desc: false });
  });
});

describe('sort in the grid query', () => {
  beforeEach(() => { useUiStore.setState(initial, true); });
  it('default adds nothing; Ngày thêm adds sort + descending; changing field resets the direction', () => {
    expect(q().sort).toBeUndefined();
    useUiStore.getState().setSortBy('added');
    expect(q()).toMatchObject({ sort: 'added', descending: true });
    useUiStore.getState().setSortDesc(false);
    expect(q()).toMatchObject({ sort: 'added', descending: false });
    useUiStore.getState().setSortBy('weight');
    expect(q()).toMatchObject({ sort: 'weight', descending: false });
  });
  it('while searching the default is Khớp (relevance); an explicit Tên/Ngày/Weight is a pure field sort; Khớp has no direction', () => {
    useUiStore.setState({ query: 'inter' });
    expect(q()).toMatchObject({ sort: 'match' });
    expect(q().descending).toBeUndefined();
    useUiStore.getState().setSortBy('family'); // tường minh, dù đúng mặc định
    expect(q()).toMatchObject({ sort: 'family', descending: false });
    useUiStore.getState().setSortBy('match');
    expect(q().sort).toBe('match'); expect(q().descending).toBeUndefined();
    useUiStore.getState().setQuery('');
    expect(useUiStore.getState().sort).toEqual(DEFAULT_SORT); // xoá chữ tìm: Khớp tự về mặc định
    expect(q().sort).toBeUndefined();
  });
  it('Gần đây and Đang bật keep their insertion order (scoped match) whatever the sort', () => {
    useUiStore.setState({ view: 'recent', query: '' }); useUiStore.getState().setSortBy('weight');
    expect(q().sort).toBe('match');
    useUiStore.setState({ view: 'active' });
    expect(q().sort).toBe('match');
  });

  it('persists through hydrate: reads ui.sort and writes changes back', async () => {
    const api = createMockApi();
    await api.setSetting(SORT_KEY, serializeSort({ by: 'weight', desc: true }));
    await useUiStore.getState().hydrate(api);
    expect(useUiStore.getState().sort).toEqual({ by: 'weight', desc: true });
    useUiStore.getState().setSortBy('added');
    await settle(600);
    expect(JSON.parse((await api.getSetting(SORT_KEY))!)).toEqual({ v: 1, by: 'added', desc: true });
  });
});

describe('sort in the View popover', () => {
  beforeEach(() => { useUiStore.setState(initial, true); });
  const open = () => { render(<ViewOptions />); fireEvent.click(screen.getByTestId('view-open')); };
  it('picks a field, flips direction with field-specific labels, shows the source line', () => {
    open();
    const sel = screen.getByTestId('view-sort-by') as HTMLSelectElement;
    expect([...sel.options].map(o => o.value)).toEqual(['family', 'added', 'released', 'file_created', 'weight']); // chưa tìm: không có "Khớp"
    expect(screen.getByTestId('view-sort-asc').textContent).toBe('A → Z');
    fireEvent.change(sel, { target: { value: 'added' } });
    expect(useUiStore.getState().sort).toEqual({ by: 'added', desc: true, explicit: true });
    expect(screen.getByTestId('view-sort-desc').getAttribute('aria-checked')).toBe('true');
    expect(screen.getByTestId('view-sort-asc').textContent).toBe('Cũ → Mới');
    fireEvent.click(screen.getByTestId('view-sort-asc'));
    expect(useUiStore.getState().sort.desc).toBe(false);
    expect(document.querySelector('.fa-viewpop-src')?.textContent).toMatch(/lần đầu thấy/);
    fireEvent.click(screen.getByTestId('view-reset'));
    expect(useUiStore.getState().sort).toEqual(DEFAULT_SORT);
  });
  it('offers "Khớp" only while searching, selected by default, with no direction buttons', () => {
    useUiStore.setState({ query: 'inter' });
    open();
    const sel = screen.getByTestId('view-sort-by') as HTMLSelectElement;
    expect([...sel.options].map(o => o.value)).toContain('match');
    expect(sel.value).toBe('match');
    expect(screen.queryByTestId('view-sort-asc')).toBeNull();
    fireEvent.change(sel, { target: { value: 'family' } });
    expect(screen.getByTestId('view-sort-asc')).toBeTruthy();
  });
  it('Gần đây / Đang bật: sorting is locked with the reason, not offered', () => {
    useUiStore.setState({ view: 'recent' });
    open();
    expect(screen.queryByTestId('view-sort-by')).toBeNull();
    expect(screen.getByTestId('view-sort-locked').textContent).toBe('Thứ tự theo lần mở gần nhất');
    expect(screen.getByTestId('view-sort').getAttribute('aria-disabled')).toBe('true');
  });
  it('Năm phát hành / Ngày tạo file: mới → cũ mặc định, vào truy vấn, kèm NoteMark (cloud_sync = cảnh báo)', () => {
    open();
    expect(screen.queryByTestId('view-sort-note')).toBeNull();
    fireEvent.change(screen.getByTestId('view-sort-by'), { target: { value: 'released' } });
    expect(useUiStore.getState().sort).toEqual({ by: 'released', desc: true, explicit: true });
    expect(q()).toMatchObject({ sort: 'released', descending: true });
    expect(screen.getByTestId('view-sort-asc').textContent).toBe('Cũ → Mới');
    expect(screen.getByTestId('view-sort-note').dataset.kind).toBe('info');
    expect(screen.getByTestId('view-sort-note').getAttribute('aria-label')).toMatch(/copyright/);
    fireEvent.change(screen.getByTestId('view-sort-by'), { target: { value: 'file_created' } });
    expect(q()).toMatchObject({ sort: 'file_created', descending: true });
    const n = screen.getByTestId('view-sort-note');
    expect(n.dataset.kind).toBe('warn');
    expect(n.getAttribute('aria-label')).toMatch(/đồng bộ đám mây/);
    fireEvent.change(screen.getByTestId('view-sort-by'), { target: { value: 'weight' } });
    expect(screen.queryByTestId('view-sort-note')).toBeNull();
  });
  it('Nhóm theo Family: checkbox in the popover, goes into the grid query, survives a sort change, and Đặt lại clears it', () => {
    open();
    const box = screen.getByTestId('view-group-family') as HTMLInputElement;
    expect(box.checked).toBe(false);
    expect(q().groupByFamily).toBeUndefined();
    fireEvent.click(box);
    expect(useUiStore.getState().sort.group).toBe(true);
    expect(q().groupByFamily).toBe(true);
    fireEvent.change(screen.getByTestId('view-sort-by'), { target: { value: 'weight' } });
    expect(useUiStore.getState().sort).toMatchObject({ by: 'weight', group: true });
    expect(q()).toMatchObject({ sort: 'weight', groupByFamily: true });
    fireEvent.click(screen.getByTestId('view-reset'));
    expect(useUiStore.getState().sort).toEqual(DEFAULT_SORT);
    expect(q().groupByFamily).toBeUndefined();
  });
  it('⌥G toggles grouping from anywhere that is not a text field', () => {
    render(<ViewOptions />);
    fireEvent.keyDown(document.body, { key: 'g', code: 'KeyG', altKey: true });
    expect(useUiStore.getState().sort.group).toBe(true);
    fireEvent.keyDown(document.body, { key: 'g', code: 'KeyG', altKey: true });
    expect(useUiStore.getState().sort.group).toBeUndefined();
    const input = document.createElement('input'); document.body.appendChild(input); input.focus();
    fireEvent.keyDown(input, { key: 'g', code: 'KeyG', altKey: true });
    expect(useUiStore.getState().sort.group).toBeUndefined(); // đang gõ chữ
    input.remove();
  });
});
