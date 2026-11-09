import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it } from 'vitest';
import { createMockApi, mockRows } from '../../../api/mock';
import { COMPARE_MAX, useUiStore } from '../../../store/ui';
import { ComparePage } from '../ComparePage';

const initial = useUiStore.getState();
const rows = mockRows();
const key = (i: number) => `${rows[i].hashHex}:${rows[i].faceIndex}`;
const fk = (i: number) => ({ hashHex: rows[i].hashHex, faceIndex: rows[i].faceIndex });
beforeEach(() => { useUiStore.setState(initial, true); });

function Host({ api }: { api: ReturnType<typeof createMockApi> }) {
  const faces = useUiStore(s => s.compare);
  return faces ? <ComparePage api={api} faces={faces} /> : <p>closed</p>;
}
async function mount(n = 3) {
  const api = createMockApi();
  useUiStore.getState().openCompare(rows.slice(0, n).map((_, i) => fk(i)));
  render(<Host api={api} />);
  await screen.findByRole('table');
  return api;
}
const heads = () => [...document.querySelectorAll('.cmp-tbl .ch')] as HTMLElement[];
const names = () => heads().map(h => h.querySelector('.t span')!.textContent);

describe('H2 · pin', () => {
  it('a pinned column jumps to the front (right after the property column) in pin order, base badge stays on the base', async () => {
    await mount(4);
    const before = names();
    act(() => { useUiStore.getState().toggleComparePin(key(3)); });
    act(() => { useUiStore.getState().toggleComparePin(key(2)); });
    const after = names();
    expect(after[0]).toBe(before[3]);
    expect(after[1]).toBe(before[2]);
    expect(after.slice(2)).toEqual([before[0], before[1]]);
    expect(heads()[2].className).toContain('base');
    expect(heads()[2].textContent).toContain('mốc');
    expect(heads()[0].className).toContain('pinned');
  });
  it('the pin button toggles and is exposed with aria-pressed; many columns can be pinned; unpin restores order', async () => {
    await mount(3);
    const btn = (i: number) => within(heads()[i]).getAllByRole('button')[0];
    fireEvent.click(btn(2));
    expect(useUiStore.getState().compareSession!.pinned).toEqual([key(2)]);
    expect(within(heads()[0]).getAllByRole('button')[0].getAttribute('aria-pressed')).toBe('true');
    fireEvent.click(btn(2)); // cột mốc (index 2 sau khi sắp lại) — pin thêm
    expect(useUiStore.getState().compareSession!.pinned).toHaveLength(2);
    fireEvent.click(btn(0)); fireEvent.click(within(heads()[0]).getAllByRole('button')[0]);
    expect(useUiStore.getState().compareSession!.pinned.length).toBeLessThan(2);
  });
  it('pin state lives in the session store (kept when the page is closed and resumed)', async () => {
    await mount(3);
    act(() => { useUiStore.getState().toggleComparePin(key(1)); useUiStore.getState().closeCompare(); });
    await screen.findByText('closed');
    act(() => { useUiStore.getState().resumeCompare(); });
    await screen.findByRole('table');
    expect(heads()[0].getAttribute('data-pinned')).toBe('true');
  });
});

describe('H3 · add / remove columns', () => {
  it('× removes a column; with two left it is disabled with an explanation', async () => {
    await mount(3);
    const x = (i: number) => within(heads()[i]).getAllByRole('button')[1];
    fireEvent.click(x(2));
    await waitFor(() => expect(heads()).toHaveLength(2));
    expect(useUiStore.getState().compare).toHaveLength(2);
    expect(x(0).getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(x(0));
    expect(heads()).toHaveLength(2);
  });
  it('the + column opens a library search; picking a font adds a column (and fonts already compared are not offered)', async () => {
    await mount(2);
    fireEvent.click(screen.getByRole('button', { name: 'Thêm font' }));
    const dlg = await screen.findByRole('dialog', { name: 'Chọn font để thêm' });
    const inCompare = new Set(heads().map(h => h.getAttribute('title')));
    const opts = await within(dlg).findAllByRole('option');
    expect(opts.length).toBeGreaterThan(0);
    for (const o of opts) expect(inCompare.has(`${o.querySelector('b')!.textContent} ${o.querySelector('span')!.textContent}`.trim())).toBe(false);
    fireEvent.click(opts[0]);
    await waitFor(() => expect(heads()).toHaveLength(3));
    expect(screen.queryByRole('dialog', { name: 'Chọn font để thêm' })).toBeNull();
    expect(useUiStore.getState().compareSession!.faces).toHaveLength(3);
  });
  it('searching filters the list', async () => {
    await mount(2);
    fireEvent.click(screen.getByRole('button', { name: 'Thêm font' }));
    const dlg = await screen.findByRole('dialog');
    await within(dlg).findAllByRole('option');
    fireEvent.change(within(dlg).getByRole('searchbox'), { target: { value: 'zzzz-no-such' } });
    expect(await within(dlg).findByText('Không có font nào khớp')).toBeTruthy();
  });
  it('at the cap the + is aria-disabled and does not open the picker', async () => {
    await mount(COMPARE_MAX);
    const add = screen.getByRole('button', { name: 'Thêm font' });
    expect(add.getAttribute('aria-disabled')).toBe('true');
    fireEvent.click(add);
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});
