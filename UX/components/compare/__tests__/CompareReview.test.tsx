import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi, mockRows } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { useUiStore } from '../../../store/ui';
import { ComparePage } from '../ComparePage';

vi.mock('../../detail/useFontFamily', () => ({ useFontFamily: () => 'F' }));
const initial = useUiStore.getState();
const rows = mockRows();
const fk = (i: number) => ({ hashHex: rows[i].hashHex, faceIndex: rows[i].faceIndex });
const key = (i: number) => `${rows[i].hashHex}:${rows[i].faceIndex}`;
beforeEach(() => { useUiStore.setState(initial, true); useNoticeStore.setState({ notices: [] }); });
afterEach(() => vi.restoreAllMocks());
function Host({ api }: { api: ReturnType<typeof createMockApi> }) {
  const faces = useUiStore(s => s.compare);
  return faces ? <ComparePage api={api} faces={faces} /> : <p>closed</p>;
}
async function mount(n = 3, api = createMockApi()) {
  useUiStore.getState().openCompare(Array.from({ length: n }, (_, i) => fk(i)));
  render(<Host api={api} />); await screen.findByRole('table');
  return api;
}
const heads = () => [...document.querySelectorAll('.cmp-tbl .ch')] as HTMLElement[];

describe('1 · Esc and IME composition', () => {
  it('Esc during composition or while typing in the specimen does not close the page; in the field it blurs', async () => {
    await mount();
    const input = screen.getAllByRole('textbox')[0] as HTMLInputElement;
    input.focus();
    fireEvent.keyDown(input, { key: 'Escape', isComposing: true, keyCode: 229 });
    expect(useUiStore.getState().compare).not.toBeNull();
    expect(document.activeElement).toBe(input);
    fireEvent.keyDown(input, { key: 'Escape' });
    expect(useUiStore.getState().compare).not.toBeNull();
    expect(document.activeElement).not.toBe(input);
    fireEvent.keyDown(document.body, { key: 'Escape', isComposing: true, keyCode: 229 });
    expect(useUiStore.getState().compare).not.toBeNull();
    fireEvent.keyDown(document.body, { key: 'Escape' });
    expect(useUiStore.getState().compare).toBeNull();
  });
});

describe('2/3 · columns change without reloading the table', () => {
  it('removing a column filters locally (no fetch, table stays); adding fetches only the new face', async () => {
    const api = await mount(3);
    const spy = vi.spyOn(api, 'compareFaces');
    const table = screen.getByRole('table');
    fireEvent.click(within(heads()[2]).getAllByRole('button').at(-1)!);
    await waitFor(() => expect(heads()).toHaveLength(2));
    expect(spy).not.toHaveBeenCalled();
    expect(screen.getByRole('table')).toBe(table);
    act(() => { useUiStore.getState().addCompareFace(fk(5)); });
    await waitFor(() => expect(heads()).toHaveLength(3));
    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy.mock.calls[0][0]).toEqual([fk(0), fk(5)]); // mốc + mặt mới, không nạp lại các cột cũ
    expect(screen.getByRole('table')).toBe(table);
    expect(screen.queryByRole('status')).toBeNull();
  });
  it('a failed add is reported and the face is dropped from the session', async () => {
    const api = await mount(2);
    vi.spyOn(api, 'compareFaces').mockRejectedValueOnce(new Error('boom'));
    act(() => { useUiStore.getState().addCompareFace(fk(5)); });
    await waitFor(() => expect(useNoticeStore.getState().notices.some(n => /boom/.test(n.text))).toBe(true));
    expect(useUiStore.getState().compareSession!.faces).toHaveLength(2);
    expect(heads()).toHaveLength(2);
  });
  it('an unreadable face is pruned from the session (no ghost column), the reason stays visible, and + is not wrongly disabled', async () => {
    const api = createMockApi();
    const real = api.compareFaces.bind(api);
    vi.spyOn(api, 'compareFaces').mockImplementation(async fs => { const d = await real(fs.slice(0, 2)); return { ...d, failed: [{ face: fs[2], reason: 'hỏng' }] }; });
    await mount(3, api);
    await waitFor(() => expect(useUiStore.getState().compareSession!.faces).toHaveLength(2));
    expect(useUiStore.getState().compare).toHaveLength(2);
    expect(screen.getByRole('alert').textContent).toMatch(/hỏng/);
    expect(screen.getByRole('button', { name: 'Thêm font' }).getAttribute('aria-disabled')).toBeNull();
  });
  it('the + is disabled from the SESSION size even when a column is missing from view', async () => {
    useUiStore.getState().openCompare(Array.from({ length: 8 }, (_, i) => fk(i)));
    expect(useUiStore.getState().addCompareFace(fk(9))).toBe(false);
  });
  it('addCompareFace failure surfaces a notice from the picker', async () => {
    await mount(2);
    fireEvent.click(screen.getByRole('button', { name: 'Thêm font' }));
    const dlg = await screen.findByRole('dialog');
    const opts = await within(dlg).findAllByRole('option');
    vi.spyOn(useUiStore.getState(), 'addCompareFace').mockReturnValue(false);
    fireEvent.click(opts[0]);
    expect(useNoticeStore.getState().notices.some(n => n.text === 'Không thêm được font này vào so sánh')).toBe(true);
  });
});

describe('9 · picker focus and toggle', () => {
  it('the + toggles the picker, Esc/close returns focus to the + button, options are buttons inside a listbox', async () => {
    await mount(2);
    const add = screen.getByRole('button', { name: 'Thêm font' });
    fireEvent.click(add);
    const dlg = await screen.findByRole('dialog');
    const list = within(dlg).getByRole('listbox');
    await within(list).findAllByRole('option');
    expect(list.querySelector('li[role="option"]')).toBeNull();
    expect(list.querySelector('button[role="option"]')).toBeTruthy();
    fireEvent.pointerDown(add); fireEvent.click(add); // bấm + khi đang mở: đóng, không mở lại
    expect(screen.queryByRole('dialog')).toBeNull();
    fireEvent.click(add);
    fireEvent.keyDown(await screen.findByRole('searchbox'), { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(add);
  });
  it('metric bars are not tab stops', async () => {
    await mount(2);
    expect(document.querySelectorAll('.cmp-pbar[tabindex]')).toHaveLength(0);
  });
});

describe('11 · overlay reseeds', () => {
  it('turning Overlay on again with nothing selected re-seeds base + next font', async () => {
    await mount(3);
    act(() => { useUiStore.getState().setCompareOverlay(true); });
    act(() => { for (const k of [...useUiStore.getState().compareSession!.overlayIds]) useUiStore.getState().toggleCompareOverlayFace(k); });
    act(() => { useUiStore.getState().setCompareOverlay(false); useUiStore.getState().setCompareOverlay(true); });
    expect(useUiStore.getState().compareSession!.overlayIds).toEqual([key(0), key(1)]);
  });
});
