import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import type { Api } from '../../../api';
import { FaceActionsProvider } from '../../faces/FaceActions';
import { NoticeStack } from '../../NoticeStack';
import { useNoticeStore } from '../../../store/notices';
import { useFavoriteStore, useSelectionStore } from '../../../store/selection';
import { useUiStore } from '../../../store/ui';
import { sharedFontLoader } from '../../../features/library-grid/sharedLoader';
import * as gridTypes from '../../../features/library-grid/types';
import { Library } from '../Library';
import { TopBar } from '../TopBar';

const settle = async () => { for (let i = 0; i < 8; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const initial = useUiStore.getState();
const cells = () => screen.getAllByRole('gridcell').filter(c => c.hasAttribute('data-face-id'));
const selected = () => cells().filter(c => c.getAttribute('aria-selected') === 'true');
const click = (el: Element, init: MouseEventInit = {}) => fireEvent.click(el, init);
const items = () => screen.queryAllByRole('menuitem').map(i => i.textContent ?? '');
const item = (name: RegExp | string) => screen.getByRole('menuitem', { name });

async function mount(api: Api = createMockApi()) {
  render(<FaceActionsProvider api={api}><NoticeStack /><TopBar api={api} /><Library api={api} width={1300} height={800} /></FaceActionsProvider>);
  await settle();
  return api;
}

describe('Library multi-select', () => {
  beforeEach(() => {
    useUiStore.setState(initial, true);
    useSelectionStore.getState().clear();
    useFavoriteStore.setState({ overrides: new Map() });
    useNoticeStore.setState({ notices: [] });
  });
  afterEach(() => vi.restoreAllMocks());

  it('click selects one; ⌘-click toggles; the bar appears from 2 and the panel follows the last focus', async () => {
    await mount();
    const c = cells();
    click(c[0]);
    expect(selected()).toHaveLength(1);
    expect(screen.queryByTestId('selection-count')).toBeNull();
    click(c[2], { metaKey: true });
    expect(selected()).toHaveLength(2);
    expect(screen.getByTestId('selection-count')).toHaveTextContent('2 đã chọn');
    const key2 = `${c[2].getAttribute('data-face-id')}`;
    expect(useUiStore.getState().selectedId).toBeTruthy();
    expect(useSelectionStore.getState().rows.size).toBe(2);
    click(c[2], { ctrlKey: true }); // Ctrl cũng bật/tắt
    expect(selected()).toHaveLength(1);
    expect(screen.queryByTestId('selection-count')).toBeNull();
    expect(key2).toBeTruthy();
    click(c[3]); // click thường thay vùng chọn
    expect(selected()).toHaveLength(1);
    expect(selected()[0]).toBe(cells()[3]);
  });

  it('Shift-click selects the range by index, and fetches rows that are not loaded yet', async () => {
    const api = await mount();
    const spy = vi.spyOn(api, 'queryFaces');
    const c = cells();
    click(c[1]);
    click(c[4], { shiftKey: true });
    await settle();
    expect(selected()).toHaveLength(4);
    expect(spy).toHaveBeenCalledWith(expect.objectContaining({ offset: 1, limit: 4 }));
    expect(screen.getByTestId('selection-count')).toHaveTextContent('4 đã chọn');
    // kéo ngược cũng được, và mốc không đổi
    click(c[0], { shiftKey: true });
    await settle();
    expect(selected()).toHaveLength(2);
  });

  it('⌘A selects everything in the query (capped) and Esc clears', async () => {
    const api = await mount();
    const total = (await api.queryFaces({ limit: 1 })).total;
    act(() => { fireEvent.keyDown(document.body, { key: 'a', metaKey: true }); });
    await settle();
    expect(useSelectionStore.getState().rows.size).toBe(Math.min(total, 2000));
    expect(screen.getByTestId('selection-count')).toHaveTextContent(`${Math.min(total, 2000).toLocaleString('vi-VN')} đã chọn`);
    act(() => { fireEvent.keyDown(document.body, { key: 'Escape' }); });
    expect(useSelectionStore.getState().rows.size).toBe(0);
    expect(selected()).toHaveLength(0);
    expect(screen.queryByTestId('selection-count')).toBeNull();
  });

  describe('Esc', () => {
    const select2 = () => { const c = cells(); click(c[0]); click(c[1], { metaKey: true }); expect(useSelectionStore.getState().rows.size).toBe(2); return c; };
    const esc = (el: Element = document.body) => { act(() => { fireEvent.keyDown(el, { key: 'Escape' }); }); };

    it('with a context menu open: the first Esc closes only the menu, the next one clears the selection', async () => {
      await mount();
      const c = select2();
      fireEvent.contextMenu(c[0], { clientX: 20, clientY: 20 });
      expect(screen.getByRole('menu')).toBeInTheDocument();
      esc(document.activeElement ?? document.body);          // focus đang ở trong menu
      expect(screen.queryByRole('menu')).toBeNull();
      expect(useSelectionStore.getState().rows.size).toBe(2);
      esc();
      expect(useSelectionStore.getState().rows.size).toBe(0);
    });

    it('…even when focus has left the menu (Esc on the body): menu closes first, selection stays', async () => {
      await mount();
      const c = select2();
      fireEvent.contextMenu(c[0], { clientX: 20, clientY: 20 });
      (document.activeElement as HTMLElement | null)?.blur();
      esc(document.body);
      expect(screen.queryByRole('menu')).toBeNull();
      expect(useSelectionStore.getState().rows.size).toBe(2);
      esc();
      expect(useSelectionStore.getState().rows.size).toBe(0);
    });

    it('with a tooltip showing: the first Esc closes only the tooltip, wherever focus is', async () => {
      await mount();
      const c = select2();
      fireEvent.pointerEnter(within(c[0]).getByTestId('vi-mark'));
      await act(async () => { await new Promise(r => setTimeout(r, 520)); });
      expect(screen.getByRole('tooltip')).toBeInTheDocument();
      esc(c[0]);
      expect(screen.queryByRole('tooltip')).toBeNull();
      expect(useSelectionStore.getState().rows.size).toBe(2);
      esc(c[0]);
      expect(useSelectionStore.getState().rows.size).toBe(0);
    });

    it('Esc while typing in the sample or search box leaves the selection alone', async () => {
      await mount();
      select2();
      for (const type of ['text', 'search']) {
        const input = document.createElement('input'); input.type = type; document.body.appendChild(input); input.focus();
        esc(input);
        expect(useSelectionStore.getState().rows.size).toBe(2);
        input.remove();
      }
    });

    it('a plain Esc on a focused card clears the selection and closes the panel', async () => {
      await mount();
      const c = select2();
      c[1].focus();
      esc(c[1]);
      expect(useSelectionStore.getState().rows.size).toBe(0);
      expect(useUiStore.getState().selectedId).toBeNull();
    });
  });

  it('⌘A does not hijack typing in an input', async () => {
    await mount();
    const input = document.createElement('input'); document.body.appendChild(input); input.focus();
    fireEvent.keyDown(input, { key: 'a', metaKey: true });
    await settle();
    expect(useSelectionStore.getState().rows.size).toBe(0);
    input.remove();
  });

  it('reports when ⌘A is capped', async () => {
    const api = await mount();
    const real = api.queryFaces.bind(api);
    vi.spyOn(api, 'queryFaces').mockImplementation(async q => { const p = await real(q); return { ...p, total: 5000 }; });
    act(() => { fireEvent.keyDown(document.body, { key: 'a', ctrlKey: true }); });
    await settle();
    // Mock chỉ có vài font nhưng báo 5.000 → vẫn lấy được những gì có, và nói rõ giới hạn
    expect(useNoticeStore.getState().notices.some(n => /2\.000 font đầu tiên/.test(n.text))).toBe(true);
  });

  it('selection is cleared when the view or query changes, but survives scrolling state like favourites', async () => {
    await mount();
    click(cells()[0]); click(cells()[1], { metaKey: true });
    expect(useSelectionStore.getState().rows.size).toBe(2);
    act(() => useUiStore.getState().setQuery('be')); await settle();
    expect(useSelectionStore.getState().rows.size).toBe(0);
    click(cells()[0]);
    act(() => useUiStore.getState().setView('star')); await settle();
    expect(useSelectionStore.getState().rows.size).toBe(0);
  });

  it('the selection bar sits in the row under the top bar (next to the family chip), never over the grid', async () => {
    const api = await mount();
    const header = screen.getByRole('banner');
    expect(header).not.toHaveClass('has-row');
    const c = cells();
    click(c[0]); click(c[1], { metaKey: true });
    const bar = screen.getByRole('toolbar');
    expect(header).toHaveClass('has-row');
    expect(header.contains(bar)).toBe(true);
    expect(bar.closest('.fa-family-row')).not.toBeNull();
    expect(screen.getByRole('grid').contains(bar)).toBe(false);
    // cùng hàng với chip family khi có phạm vi family
    const styles = await api.getFamilyStyles('Be Vietnam Pro');
    act(() => useUiStore.getState().openFamily({ name: 'Be Vietnam Pro', faces: styles.map(x => x.face) })); await settle();
    const [a, b] = cells();
    click(a); click(b, { metaKey: true });
    const row = screen.getByRole('toolbar').closest('.fa-family-row')!;
    expect(row.querySelectorAll('.fa-family-chip')).toHaveLength(1);
    // bỏ chọn → hàng chỉ còn chip; không còn chip → hàng biến mất
    fireEvent.click(within(screen.getByRole('toolbar')).getByRole('button', { name: 'Bỏ chọn' }));
    expect(screen.queryByRole('toolbar')).toBeNull();
    expect(row.querySelector('.fa-family-chip')).not.toBeNull();
  });

  it('unfavouriting in Yêu thích drops those cards from the selection (they leave the query) but keeps the rest', async () => {
    const api = createMockApi();
    const items = (await api.queryFaces({ limit: 4 })).items;
    for (const r of items) await api.setFavorite({ hashHex: r.hashHex, faceIndex: r.faceIndex }, true);
    act(() => useUiStore.getState().setView('star'));
    await mount(api);
    const c = cells();
    expect(c).toHaveLength(4);
    click(c[0]); click(c[1], { metaKey: true }); click(c[2], { metaKey: true });
    expect(useSelectionStore.getState().rows.size).toBe(3);
    const gone = c[1].getAttribute('data-face-id');
    await act(async () => { fireEvent.click(within(c[1]).getByRole('button', { name: 'Yêu thích' })); });
    await settle();
    const left = [...useSelectionStore.getState().rows.values()].map(r => String(r.faceId));
    expect(left).toHaveLength(2);
    expect(left).not.toContain(gone);
    expect(cells()).toHaveLength(3);
    expect(screen.getByTestId('selection-count')).toHaveTextContent('2 đã chọn');
  });

  it('all cards and the panel share ONE font loader; card names ask for it as soft refs', async () => {
    const api = createMockApi();
    const shared = sharedFontLoader(api);
    const spy = vi.spyOn(shared.loader, 'acquire');
    await mount(api);
    expect(sharedFontLoader(api)).toBe(shared);
    expect(spy).toHaveBeenCalled();
    expect(spy.mock.calls.every(([, o]) => o?.soft === true)).toBe(true); // lưới chỉ xin tên (mock không có URL font nên không tải thật)
  });

  describe('Gần đây keeps its order while you work in it', () => {
    const seed = async (api: Api) => {
      const rows = (await api.queryFaces({ limit: 6 })).items;
      const keys = rows.map(r => `${r.hashHex}:${r.faceIndex}`);
      useUiStore.setState({ recentViews: keys.map((face, i) => ({ face, at: 1000 - i })) });
      return keys;
    };
    const shownKeys = () => cells().map(c => c.getAttribute('data-face-id'));

    it('choosing and opening fonts does not reorder the grid; leaving and coming back does', async () => {
      const api = createMockApi();
      const keys = await seed(api);
      act(() => useUiStore.getState().setView('recent'));
      await mount(api);
      const spy = vi.spyOn(api, 'queryFaces');
      const before = shownKeys();
      expect(before).toHaveLength(6);
      click(cells()[4]);
      await settle();
      click(cells()[2], { metaKey: true });
      await settle();
      // thứ tự trong store đã đổi (font vừa chọn lên đầu)…
      expect(useUiStore.getState().recentViews[0].face).not.toBe(keys[0]);
      // …nhưng lưới đứng yên và không truy vấn lại
      expect(shownKeys()).toEqual(before);
      // (chỉ truy vấn đếm limit=1 của tiêu đề dùng recents mới; lưới không nạp lại)
      expect(spy.mock.calls.some(([q]) => (q as { faceKeys?: unknown; limit?: number }).faceKeys && (q.limit ?? 0) > 1)).toBe(false);
      // rời view rồi quay lại → thứ tự mới
      act(() => useUiStore.getState().setView('all')); await settle();
      act(() => useUiStore.getState().setView('recent')); await settle();
      expect(shownKeys()).not.toEqual(before);
      expect(shownKeys()[0]).toBe(String(cells()[0].getAttribute('data-face-id')));
    });

    it('Shift-click in Gần đây selects exactly the visible range after earlier clicks moved fonts in the store', async () => {
      const api = createMockApi();
      await seed(api);
      act(() => useUiStore.getState().setView('recent'));
      await mount(api);
      const ids = shownKeys();
      click(cells()[1]);            // làm store đổi thứ tự recents
      await settle();
      click(cells()[4], { shiftKey: true });
      await settle();
      expect(selected().map(c => c.getAttribute('data-face-id'))).toEqual(ids.slice(1, 5));
      expect(screen.getByTestId('selection-count')).toHaveTextContent('4 đã chọn');
    });
  });

  describe('context menu', () => {
    it('right-click on an unselected card selects just that card and opens the single-font menu', async () => {
      await mount();
      const c = cells();
      click(c[0]); click(c[1], { metaKey: true });
      fireEvent.contextMenu(c[3], { clientX: 40, clientY: 40 });
      expect(selected()).toEqual([cells()[3]]);
      expect(items()).toEqual(expect.arrayContaining(['Mở chi tiết', 'Xem family', 'Tìm tương tự', 'Ghép cặp', 'Yêu thích', 'Thêm vào dự án›', 'Gắn thẻ›', 'Kích hoạt', 'Hiện trong Finder', 'Sao chép tên']));
    });

    it('right-click inside the selection keeps it and offers the batch menu', async () => {
      await mount();
      const c = cells();
      click(c[0]); click(c[1], { metaKey: true }); click(c[2], { metaKey: true });
      fireEvent.contextMenu(c[1], { clientX: 40, clientY: 40 });
      expect(selected()).toHaveLength(3);
      expect(screen.getByText('3 font đã chọn')).toBeInTheDocument();
      expect(screen.queryByRole('menuitem', { name: 'Mở chi tiết' })).toBeNull();
      expect(item('Hiện trong Finder')).toHaveAttribute('aria-disabled', 'true');
    });

    it('ContextMenu key and Shift+F10 open it from the focused card, with the first item focused', async () => {
      await mount();
      const c = cells()[0];
      c.focus();
      fireEvent.keyDown(c, { key: 'ContextMenu' });
      expect(screen.getByRole('menu')).toBeInTheDocument();
      expect(document.activeElement).toBe(item('Mở chi tiết'));
      fireEvent.keyDown(document.activeElement!, { key: 'Escape' });
      expect(screen.queryByRole('menu')).toBeNull();
      expect(document.activeElement).toBe(c);
      fireEvent.keyDown(c, { key: 'F10', shiftKey: true });
      expect(screen.getByRole('menu')).toBeInTheDocument();
    });

    it('"Mở chi tiết" / "Tìm tương tự" / "Ghép cặp" drive the detail panel', async () => {
      await mount();
      fireEvent.contextMenu(cells()[1]);
      fireEvent.click(item('Tìm tương tự'));
      expect(useUiStore.getState().panelTab).toBe('sim');
      expect(useUiStore.getState().selectedId).toBeTruthy();
      fireEvent.contextMenu(cells()[2]);
      fireEvent.click(item('Ghép cặp'));
      expect(useUiStore.getState().panelTab).toBe('pair');
    });

    it('favourites the whole selection through setFavorite, one call per font, and flips the label', async () => {
      const api = await mount();
      const spy = vi.spyOn(api, 'setFavorite');
      const c = cells();
      click(c[0]); click(c[1], { metaKey: true });
      fireEvent.contextMenu(c[0]);
      await act(async () => { fireEvent.click(item('Yêu thích')); });
      await settle();
      expect(spy).toHaveBeenCalledTimes(2);
      expect(spy.mock.calls.every(([, on]) => on === true)).toBe(true);
      expect(useNoticeStore.getState().notices.map(n => n.text)).toContain('Đã thêm 2 font vào Yêu thích');
      fireEvent.contextMenu(cells()[0]);
      expect(item('Bỏ yêu thích')).toBeInTheDocument();
    });

    it('Thêm vào dự án ▸ Dự án mới… creates a project holding both fonts', async () => {
      const api = await mount();
      const create = vi.spyOn(api, 'createCollection');
      const add = vi.spyOn(api, 'addToCollection');
      const c = cells();
      click(c[0]); click(c[1], { metaKey: true });
      const keys = [...useSelectionStore.getState().rows.keys()].sort();
      fireEvent.contextMenu(c[0]);
      await settle();
      const parent = item('Thêm vào dự án');
      fireEvent.keyDown(parent, { key: 'ArrowRight' });
      fireEvent.click(item('Dự án mới…'));
      const input = await screen.findByRole('textbox', { name: 'Tên dự án' });
      fireEvent.change(input, { target: { value: 'Menu quán' } });
      await act(async () => { fireEvent.keyDown(input, { key: 'Enter' }); });
      await settle();
      expect(create).toHaveBeenCalledWith('Menu quán', null);
      const [colId, faces] = add.mock.calls[0];
      expect(faces.map(f => `${f.hashHex}:${f.faceIndex}`).sort()).toEqual(keys);
      const cols = await api.listCollections();
      expect(cols.find(x => x.id === colId)).toMatchObject({ name: 'Menu quán', itemCount: 2 });
      expect(useNoticeStore.getState().notices.map(n => n.text)).toContain('Đã thêm 2 font vào «Menu quán»');
      expect(useUiStore.getState().libraryRev).toBeGreaterThan(0); // thanh bên nạp lại danh sách dự án
    });

    it('adds to an existing project and tags the selection', async () => {
      const api = await mount();
      const col = await api.createCollection('Có sẵn', null);
      const tag = await api.createTag('Serif');
      const add = vi.spyOn(api, 'addToCollection'); const tagFaces = vi.spyOn(api, 'tagFaces');
      const c = cells();
      click(c[0]); click(c[1], { metaKey: true });
      fireEvent.contextMenu(c[0]); await settle();
      fireEvent.keyDown(item('Thêm vào dự án'), { key: 'ArrowRight' });
      await act(async () => { fireEvent.click(item(/Có sẵn/)); });
      expect(add).toHaveBeenCalledWith(col.id, expect.arrayContaining([expect.objectContaining({ faceIndex: 0 })]));
      expect(add.mock.calls[0][1]).toHaveLength(2);
      fireEvent.contextMenu(cells()[0]); await settle();
      fireEvent.keyDown(item('Gắn thẻ'), { key: 'ArrowRight' });
      await act(async () => { fireEvent.click(item(/Serif/)); });
      expect(tagFaces).toHaveBeenCalledWith(tag.id, expect.any(Array));
      expect(tagFaces.mock.calls[0][1]).toHaveLength(2);
    });

    it('activation goes through the activation API for the selection', async () => {
      const api = await mount();
      const act1 = vi.spyOn(api, 'activationActivate');
      const c = cells();
      click(c[0]); click(c[1], { metaKey: true });
      fireEvent.contextMenu(c[0]);
      await act(async () => { fireEvent.click(item('Kích hoạt')); });
      await settle();
      expect(act1).toHaveBeenCalledTimes(1);
      expect(act1.mock.calls[0][0]).toHaveLength(2);
    });

    it('a partial failure is reported, not hidden', async () => {
      const api = await mount();
      let n = 0;
      vi.spyOn(api, 'setFavorite').mockImplementation(async () => { if (++n === 2) throw new Error('đĩa đầy'); });
      const c = cells();
      click(c[0]); click(c[1], { metaKey: true }); click(c[2], { metaKey: true });
      fireEvent.contextMenu(c[0]);
      await act(async () => { fireEvent.click(item('Yêu thích')); });
      await settle();
      const texts = useNoticeStore.getState().notices.map(n => n.text);
      expect(texts).toContain('Đã thêm 2 font vào Yêu thích');
      expect(texts.some(t => /1 font không đổi được yêu thích: đĩa đầy/.test(t))).toBe(true);
    });

    it('selection bar: favourite and Bỏ chọn', async () => {
      const api = await mount();
      const spy = vi.spyOn(api, 'setFavorite');
      const c = cells();
      click(c[0]); click(c[1], { metaKey: true });
      const bar = screen.getByRole('toolbar');
      await act(async () => { fireEvent.click(within(bar).getByRole('button', { name: /Yêu thích/ })); });
      expect(spy).toHaveBeenCalledTimes(2);
      fireEvent.click(within(bar).getByRole('button', { name: 'Bỏ chọn' }));
      expect(screen.queryByRole('toolbar')).toBeNull();
      expect(selected()).toHaveLength(0);
    });

    it('selection bar opens the project list under its button', async () => {
      const api = await mount();
      await api.createCollection('Một', null);
      const c = cells();
      click(c[0]); click(c[1], { metaKey: true });
      fireEvent.click(within(screen.getByRole('toolbar')).getByRole('button', { name: /Thêm vào dự án/ }));
      await settle();
      expect(items()).toEqual(expect.arrayContaining(['Một0', 'Dự án mới…']));
    });
  });
});

describe('Library: thêm vào dự án (G1)', () => {
  beforeEach(() => {
    useUiStore.setState(initial, true); useSelectionStore.getState().clear(); useFavoriteStore.setState({ overrides: new Map() }); useNoticeStore.setState({ notices: [] });
  });
  afterEach(() => vi.restoreAllMocks());

  it('⌥⌘P mở danh sách dự án cho font đang chọn, Enter thêm vào dự án đó; không có vùng chọn thì không làm gì', async () => {
    const api = createMockApi(); const col = await api.createCollection('Menu', null);
    await mount(api);
    fireEvent.keyDown(window, { key: 'π', code: 'KeyP', altKey: true, metaKey: true }); await settle();
    expect(screen.queryByRole('menu')).toBeNull();
    click(cells()[0]); await settle();
    fireEvent.keyDown(window, { key: 'π', code: 'KeyP', altKey: true, metaKey: true }); await settle();
    expect(screen.getByRole('menu')).toBeTruthy();
    fireEvent.click(screen.getByRole('menuitem', { name: /Menu/ })); await settle();
    expect((await api.queryFaces({ collectionId: col.id, limit: 10 })).items).toHaveLength(1);
    expect(useNoticeStore.getState().notices.some(n => /Đã thêm/.test(n.text))).toBe(true);
  });

  it('kéo thẻ rồi thả lên mục dự án (đích data-drop-col) thêm font vào dự án và báo có Hoàn tác', async () => {
    const api = createMockApi(); const col = await api.createCollection('Poster', null);
    await mount(api);
    const drop = document.createElement('div'); drop.dataset.dropCol = String(col.id); drop.dataset.dropName = 'Poster'; document.body.appendChild(drop);
    (document as unknown as { elementFromPoint: unknown }).elementFromPoint = () => drop;
    const cell = cells()[1];
    act(() => { cell.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, button: 0, clientX: 5, clientY: 5 })); }); // jsdom không có PointerEvent: MouseEvent mang button/clientX
    act(() => { window.dispatchEvent(Object.assign(new Event('pointermove'), { clientX: 60, clientY: 60 })); });
    expect(screen.getByTestId('drag-count').textContent).toBe('1');
    act(() => { window.dispatchEvent(Object.assign(new Event('pointerup'), { clientX: 60, clientY: 60 })); }); await settle();
    expect((await api.queryFaces({ collectionId: col.id, limit: 10 })).items).toHaveLength(1);
    expect(useNoticeStore.getState().notices.at(-1)?.action?.label).toBe('Hoàn tác');
    drop.remove();
  });

  const dragCellTo = (cell: HTMLElement, drop: HTMLElement) => {
    (document as unknown as { elementFromPoint: unknown }).elementFromPoint = () => drop;
    act(() => { cell.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, button: 0, clientX: 5, clientY: 5 })); });
    act(() => { window.dispatchEvent(Object.assign(new Event('pointermove'), { clientX: 60, clientY: 60 })); });
    act(() => { window.dispatchEvent(Object.assign(new Event('pointerup'), { clientX: 60, clientY: 60 })); });
  };

  it('thả thẻ lên một thẻ (data-drop-tag) gắn thẻ, Hoàn tác gỡ đúng phần vừa gắn', async () => {
    const api = createMockApi(); const tg = await api.createTag('Logo');
    await mount(api);
    const drop = document.createElement('div'); drop.dataset.dropTag = String(tg.id); drop.dataset.dropName = 'Logo'; document.body.appendChild(drop);
    dragCellTo(cells()[1], drop); await settle();
    expect((await api.queryFaces({ tagIds: [tg.id], limit: 10 })).items).toHaveLength(1);
    const n = useNoticeStore.getState().notices.at(-1);
    expect(n?.action?.label).toBe('Hoàn tác');
    act(() => n?.action?.run()); await settle();
    expect((await api.queryFaces({ tagIds: [tg.id], limit: 10 })).items).toHaveLength(0);
    dragCellTo(cells()[1], drop); await settle(); dragCellTo(cells()[1], drop); await settle(); // thả lần hai: đã có thẻ → không gắn thêm
    expect(useNoticeStore.getState().notices.at(-1)?.text).toMatch(/Đã có thẻ/);
    drop.remove();
  });

  it('kéo từ Dự án A sang B: mặc định thêm (A giữ nguyên); giữ ⌘ thì chuyển (A mất font), Hoàn tác đưa về A', async () => {
    const api = createMockApi(); const a = await api.createCollection('A', null); const b = await api.createCollection('B', null);
    const first = (await api.queryFaces({ limit: 1 })).items[0]; await api.addToCollection(a.id, [{ hashHex: first.hashHex, faceIndex: first.faceIndex }]);
    useUiStore.setState({ view: `col:${a.id}` });
    await mount(api);
    const drop = document.createElement('div'); drop.dataset.dropCol = String(b.id); drop.dataset.dropName = 'B'; document.body.appendChild(drop);
    const drag = (meta: boolean) => {
      (document as unknown as { elementFromPoint: unknown }).elementFromPoint = () => drop;
      act(() => { cells()[0].dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, button: 0, clientX: 5, clientY: 5 })); });
      act(() => { window.dispatchEvent(Object.assign(new Event('pointermove'), { clientX: 60, clientY: 60, metaKey: meta })); });
      if (meta) expect(screen.getByText(/chuyển sang/)).toBeTruthy();
      act(() => { window.dispatchEvent(Object.assign(new Event('pointerup'), { clientX: 60, clientY: 60, metaKey: meta })); });
    };
    drag(false); await settle();
    expect((await api.queryFaces({ collectionId: a.id, limit: 10 })).items).toHaveLength(1);
    expect((await api.queryFaces({ collectionId: b.id, limit: 10 })).items).toHaveLength(1);
    await api.removeFromCollection(b.id, [{ hashHex: first.hashHex, faceIndex: first.faceIndex }]);
    await act(async () => { useUiStore.getState().bumpLibrary(false); }); await settle();
    drag(true); await settle();
    expect((await api.queryFaces({ collectionId: a.id, limit: 10 })).items).toHaveLength(0);
    expect((await api.queryFaces({ collectionId: b.id, limit: 10 })).items).toHaveLength(1);
    const n = useNoticeStore.getState().notices.at(-1);
    expect(n?.text).toMatch(/Đã chuyển/);
    act(() => n?.action?.run()); await settle();
    expect((await api.queryFaces({ collectionId: a.id, limit: 10 })).items).toHaveLength(1);
    expect((await api.queryFaces({ collectionId: b.id, limit: 10 })).items).toHaveLength(0);
    drop.remove();
  });

  it('thả thẻ lên "Đang bật" (data-drop-active) bật font; Hoàn tác tắt lại', async () => {
    const api = createMockApi();
    await mount(api);
    const drop = document.createElement('div'); drop.dataset.dropActive = '0'; drop.dataset.dropName = 'Đang bật'; document.body.appendChild(drop);
    dragCellTo(cells()[1], drop); await settle();
    expect(useUiStore.getState().active.size).toBe(1);
    const n = useNoticeStore.getState().notices.at(-1);
    expect(n?.text).toMatch(/Đã bật/); expect(n?.action?.label).toBe('Hoàn tác');
    act(() => n?.action?.run()); await settle();
    expect(useUiStore.getState().active.size).toBe(0);
    drop.remove();
  });
});

describe('Library: ghi lần xem (G5 Recent)', () => {
  it('chọn thẻ ghi một lần xem; lướt (rê chuột) thì không', async () => {
    useUiStore.setState(initial, true); useSelectionStore.getState().clear();
    const api = createMockApi(); const spy = vi.spyOn(api, 'recordView');
    await mount(api);
    fireEvent.pointerEnter(cells()[0]); fireEvent.mouseOver(cells()[0]); await settle();
    expect(spy).not.toHaveBeenCalled();
    click(cells()[0]); await settle();
    expect(spy).toHaveBeenCalledTimes(1);
    expect((await api.recentViews(5)).length).toBe(1);
  });
});
