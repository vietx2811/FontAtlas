import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { useScanStore } from '../../../store/scan';
import { useUiStore } from '../../../store/ui';
import { useFaceDragStore } from '../../../store/faceDrag';
import { Sidebar, SidebarToggle } from '../Sidebar';
import { tipOf } from '../../../test/tip';

const initial = useUiStore.getState();
const settle = async () => { for (let i = 0; i < 5; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
async function mount(over: Partial<React.ComponentProps<typeof Sidebar>> = {}) {
  const api = createMockApi();
  const r = render(<Sidebar api={api} pickFolder={async () => null} {...over} />);
  await settle();
  return { api, ...r };
}

describe('Sidebar', () => {
  beforeEach(() => useUiStore.setState(initial, true));

  it('lists nav items with counts and marks the current view', async () => {
    await mount();
    const all = screen.getByRole('button', { name: /Tất cả/ });
    expect(all.getAttribute('aria-current')).toBe('true');
    expect(within(all).getByText(/^\d+$/).textContent).not.toBe('0');
    fireEvent.click(screen.getByRole('button', { name: /Yêu thích/ }));
    expect(useUiStore.getState().view).toBe('star');
    fireEvent.click(screen.getByRole('button', { name: /Đang bật/ }));
    expect(useUiStore.getState().view).toBe('active');
  });

  it('shows sources with face counts, and strikes through an unplugged drive', async () => {
    await mount();
    const src = screen.getByRole('button', { name: /Dự án khách/ });
    fireEvent.click(src);
    expect(useUiStore.getState().view).toBe('src:2');
    const gone = screen.getByRole('button', { name: /Ổ ngoài/ });
    expect(gone.getAttribute('data-unavailable')).toBe('true');
    expect(await tipOf(gone)).toMatch(/chưa kết nối/i);
  });

  it('leaves Settings when selecting a sidebar view, including the current view', async () => {
    await mount();
    for (const [name, view] of [[/Tất cả/, 'all'], [/Yêu thích/, 'star'], [/Dự án khách/, 'src:2']] as const) {
      act(() => useUiStore.getState().setSettingsOpen(true));
      fireEvent.click(screen.getByRole('button', { name }));
      expect(useUiStore.getState().view).toBe(view);
      expect(useUiStore.getState().settingsOpen).toBe(false);
    }
  });

  it('leaves Settings when selecting a tag filter', async () => {
    const api = createMockApi();
    await api.createTag('serif');
    render(<Sidebar api={api} pickFolder={async () => null} />);
    await settle();
    act(() => useUiStore.getState().setSettingsOpen(true));
    fireEvent.click(screen.getByRole('button', { name: 'serif' }));
    expect(useUiStore.getState().filters.tagIds).toHaveLength(1);
    expect(useUiStore.getState().settingsOpen).toBe(false);
  });

  it('sections collapse with aria-expanded and persist through the store', async () => {
    await mount();
    const head = screen.getByRole('button', { name: /^Nguồn/ });
    expect(head.getAttribute('aria-expanded')).toBe('true');
    fireEvent.click(head);
    expect(head.getAttribute('aria-expanded')).toBe('false');
    expect(useUiStore.getState().openSections.src).toBe(false);
  });

  it('creates a collection and selects it', async () => {
    await mount();
    fireEvent.click(screen.getByRole('button', { name: 'Thêm dự án' }));
    const input = screen.getByRole('textbox', { name: 'Tên dự án' });
    fireEvent.change(input, { target: { value: 'Menu quán' } });
    fireEvent.keyDown(input, { key: 'Enter', currentTarget: { value: 'Menu quán' } });
    await settle();
    expect(screen.getByRole('button', { name: /^Menu quán/ })).toBeTruthy();
    expect(useUiStore.getState().view).toMatch(/^col:/);
  });

  it('renames and deletes a collection from its menu', async () => {
    const api = createMockApi();
    const c = await api.createCollection('Cũ', null);
    render(<Sidebar api={api} pickFolder={async () => null} />);
    await settle();
    useUiStore.getState().setView(`col:${c.id}`);
    fireEvent.click(screen.getByRole('button', { name: 'Tuỳ chọn Cũ' }));
    fireEvent.click(screen.getByRole('menuitem', { name: 'Xoá dự án…' }));
    await settle();
    expect(screen.getByRole('button', { name: /^Cũ/ })).toBeTruthy(); // chưa xoá: đang hỏi xác nhận
    fireEvent.click(screen.getByTestId('smenu-confirm'));
    await settle();
    expect(screen.queryByRole('button', { name: /^Cũ/ })).toBeNull();
    expect(useUiStore.getState().view).toBe('all');
  });

  it('tag chips toggle the tag filter', async () => {
    const api = createMockApi();
    const t = await api.createTag('serif');
    render(<Sidebar api={api} pickFolder={async () => null} />);
    await settle();
    fireEvent.click(screen.getByRole('button', { name: 'serif' }));
    expect(useUiStore.getState().filters.tagIds).toEqual([t.id]);
    fireEvent.click(screen.getByRole('button', { name: 'serif' }));
    expect(useUiStore.getState().filters.tagIds).toBeUndefined();
  });

  it('adds a font folder through the picker and switches to the new source', async () => {
    const pickFolder = vi.fn(async () => '/Users/me/NewFonts');
    await mount({ pickFolder });
    fireEvent.click(screen.getByRole('button', { name: /Thêm font/ }));
    await settle();
    expect(pickFolder).toHaveBeenCalled();
    expect(screen.getByRole('button', { name: /NewFonts/ })).toBeTruthy();
    expect(useUiStore.getState().view).toMatch(/^src:/);
  });

  it('D4: a folder inside an existing source opens a dialog with both paths and offers a collection instead', async () => {
    const { api } = await mount({ pickFolder: async () => '/Users/me/Fonts/src1/Khách A' });
    fireEvent.click(screen.getByRole('button', { name: /Thêm font/ }));
    await settle();
    const dlg = screen.getByRole('dialog', { name: /nằm trong một nguồn đã có/ });
    expect(within(dlg).getByText('/Users/me/Fonts/src1/Khách A')).toBeTruthy();
    expect(within(dlg).getByText('/Users/me/Fonts/src1')).toBeTruthy();
    expect(dlg.textContent).toMatch(/73 font/);
    expect(dlg.textContent).toMatch(/tạo một collection/i);
    fireEvent.click(within(dlg).getByRole('button', { name: 'Tạo collection “Khách A”' }));
    await settle();
    expect(screen.queryByRole('dialog')).toBeNull();
    expect((await api.listCollections()).map(c => c.name)).toContain('Khách A');
    expect(useUiStore.getState().view).toMatch(/^col:/);
  });

  it('D4: Esc and Huỷ close the dialog without changing anything', async () => {
    const { api } = await mount({ pickFolder: async () => '/Users/me/Fonts/src1/x' });
    fireEvent.click(screen.getByRole('button', { name: /Thêm font/ }));
    await settle();
    fireEvent.keyDown(screen.getByRole('dialog'), { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: /Thêm font/ }));
    await settle();
    fireEvent.click(screen.getByRole('button', { name: 'Huỷ' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(await api.listCollections()).toEqual([]);
  });

  it('D4: a folder that contains an existing source can replace it (child removed, parent added)', async () => {
    const api = createMockApi();
    const child = await api.addSource('/Volumes/Ổ/Fonts/Khách A');
    render(<Sidebar api={api} pickFolder={async () => '/Volumes/Ổ/Fonts'} />);
    await settle();
    fireEvent.click(screen.getByRole('button', { name: /Thêm font/ }));
    await settle();
    const dlg = screen.getByRole('dialog', { name: /chứa nguồn/ });
    expect(dlg.textContent).toMatch(/Khách A/);
    fireEvent.click(within(dlg).getByRole('button', { name: 'Thay nguồn cũ bằng thư mục này' }));
    await settle();
    const names = (await api.listSources()).map(s => s.path);
    expect(names).toContain('/Volumes/Ổ/Fonts');
    expect(names).not.toContain(child.path);
    expect(useUiStore.getState().view).toMatch(/^src:/);
  });

  it('D4: the exact same path is not a dialog, just a note that it is already there', async () => {
    await mount({ pickFolder: async () => '/Users/me/Fonts/src1' });
    fireEvent.click(screen.getByRole('button', { name: /Thêm font/ }));
    await settle();
    expect(screen.queryByRole('dialog')).toBeNull();
    const note = screen.getByRole('status');
    expect(note.textContent).toMatch(/Đã có/);
    fireEvent.click(within(note).getByRole('button', { name: /Xem nguồn/ }));
    expect(useUiStore.getState().view).toBe('src:1');
  });

  it('cancelled picker does nothing', async () => {
    const { api } = await mount();
    const spy = vi.spyOn(api, 'addSource');
    fireEvent.click(screen.getByRole('button', { name: /Thêm font/ }));
    await settle();
    expect(spy).not.toHaveBeenCalled();
  });

  it('collapses to icons only with the toggle, keeping labels as accessible names', async () => {
    await mount();
    render(<SidebarToggle />);
    fireEvent.click(screen.getByRole('button', { name: 'Thu gọn thanh bên' }));
    expect(useUiStore.getState().sidebarCollapsed).toBe(true);
    expect(screen.getByTestId('sidebar').getAttribute('data-collapsed')).toBe('true');
    expect(screen.getByRole('button', { name: /Tất cả/ })).toBeTruthy();
  });

  it('theme toggle sets data-theme on the root', async () => {
    await mount();
    fireEvent.click(screen.getByRole('button', { name: /Giao diện/ }));
    expect(['light', 'dark']).toContain(document.documentElement.dataset.theme);
    document.documentElement.removeAttribute('data-theme');
  });

  it('refreshes counts when a scan finishes, and shows scan progress in the footer', async () => {
    const { api } = await mount();
    const spy = vi.spyOn(api, 'listSources');
    act(() => useScanStore.getState().bump());
    await settle();
    expect(spy).toHaveBeenCalled();
    act(() => useScanStore.setState({ scanning: true, written: 120 }));
    expect(screen.getByRole('status', { name: 'Đang quét' }).textContent).toMatch(/Đang quét · 120 font/);
    act(() => useScanStore.setState({ scanning: false }));
    expect(screen.queryByRole('status', { name: 'Đang quét' })).toBeNull();
  });

  it('opens the folder picker when another part of the UI asks for it', async () => {
    const pickFolder = vi.fn(async () => null);
    await mount({ pickFolder });
    expect(pickFolder).not.toHaveBeenCalled();
    act(() => useUiStore.getState().requestAddFolder());
    await settle();
    expect(pickFolder).toHaveBeenCalledTimes(1);
  });

  it('a source that needs access is marked in the list (title says so)', async () => {
    await mount();
    act(() => useScanStore.setState({ access: { 3: 'denied' } }));
    expect(await tipOf(screen.getByRole('button', { name: /Ổ ngoài/ }))).toBe('Ổ ngoài: cần quyền truy cập');
    act(() => useScanStore.setState({ access: {} }));
  });

  describe('files still in the cloud (not downloaded)', () => {
    it('a source whose last scan skipped placeholders says how many, with a tooltip on how to download them', async () => {
      await mount();
      const line = screen.getByTestId('not-downloaded');
      expect(line.textContent).toBe('12 font chưa tải về');
      const tip = await tipOf(line);
      expect(tip).toMatch(/Finder/);
      expect(tip).toMatch(/Tải về/);
      expect(within(line).queryByRole('button')).toBeNull(); // không có nút tự tải
      expect(screen.getAllByTestId('not-downloaded')).toHaveLength(1); // chỉ nguồn có file chưa tải
    });

    it('a cloud source warns with the real share and the Finder advice; a local source has no hint', async () => {
      await mount();
      const hints = screen.getAllByTestId('cloud-hint');
      expect(hints).toHaveLength(1);
      const tip = await tipOf(hints[0]);
      expect(tip).toMatch(/^90% font của nguồn này chưa tải về máy/);
      expect(tip).toMatch(/Always keep on this device/);
    });

    it('while scanning, the footer counts them too, and drops the line when the scan ends', async () => {
      await mount();
      act(() => useScanStore.setState({ scanning: true, written: 40, cloud: { 2: 7 } }));
      const status = screen.getByRole('status', { name: 'Đang quét' });
      expect(within(status).getByTestId('scan-not-downloaded').textContent).toBe('7 font chưa tải về');
      act(() => useScanStore.setState({ scanning: false, cloud: {} }));
      expect(screen.queryByTestId('scan-not-downloaded')).toBeNull();
    });
  });

  it('shows "Trùng lặp" with the group count, hides it at 0, and opens the duplicates view', async () => {
    const api = createMockApi();
    const spy = vi.spyOn(api, 'duplicateSummary');
    const r = render(<Sidebar api={api} pickFolder={async () => null} />);
    await settle();
    const item = screen.getByRole('button', { name: /Trùng lặp/ });
    expect(within(item).getByText(String((await spy.mock.results[0].value).total))).toBeTruthy();
    fireEvent.click(item);
    expect(useUiStore.getState().view).toBe('dups');
    expect(screen.getByRole('button', { name: /Trùng lặp/ }).getAttribute('aria-current')).toBe('true');
    r.unmount();
    useUiStore.getState().setView('all');
    spy.mockResolvedValue({ sameFile: 0, versions: 0, formats: 0, total: 0 });
    render(<Sidebar api={api} pickFolder={async () => null} />);
    await settle();
    expect(screen.queryByRole('button', { name: /Trùng lặp/ })).toBeNull();
  });

  it('keeps the item while the current view is duplicates even if the count drops to 0 (so the user can leave)', async () => {
    useUiStore.getState().setView('dups');
    const api = createMockApi();
    vi.spyOn(api, 'duplicateSummary').mockResolvedValue({ sameFile: 0, versions: 0, formats: 0, total: 0 });
    render(<Sidebar api={api} pickFolder={async () => null} />);
    await settle();
    expect(screen.getByRole('button', { name: /Trùng lặp/ })).toBeTruthy();
  });

  it('H4: shows "Đang so sánh · N" only while a compare session exists, keeps it after leaving the page, and resumes it', async () => {
    await mount();
    expect(screen.queryByRole('button', { name: /Đang so sánh/ })).toBeNull();
    const f = (n: number) => ({ hashHex: String(n), faceIndex: 0 });
    act(() => { useUiStore.getState().openCompare([f(1), f(2), f(3)]); });
    const item = screen.getByRole('button', { name: /Đang so sánh · 3/ });
    expect(item.getAttribute('aria-current')).toBe('true');
    fireEvent.click(screen.getByRole('button', { name: /Yêu thích/ })); // rời Compare
    expect(useUiStore.getState().compare).toBeNull();
    const again = screen.getByRole('button', { name: /Đang so sánh · 3/ });
    expect(again.getAttribute('aria-current')).toBeNull();
    fireEvent.click(again);
    expect(useUiStore.getState().compare).toHaveLength(3);
  });

  describe('context menu (C6)', () => {
    it('right click opens the menu of the item WITHOUT changing the view; the item is marked; Esc returns focus to it', async () => {
      const api = createMockApi();
      const c = await api.createCollection('Dự án A', null);
      render(<Sidebar api={api} pickFolder={async () => null} />); await settle();
      const item = screen.getByRole('button', { name: /^Dự án A/ });
      fireEvent.contextMenu(item, { clientX: 20, clientY: 40 }); await settle();
      expect(screen.getByRole('menu', { name: 'Menu Dự án A' })).toBeTruthy();
      expect(item.getAttribute('data-context')).toBe('true');
      expect(useUiStore.getState().view).toBe('all');
      fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' }); await settle();
      expect(screen.queryByRole('menu')).toBeNull();
      expect(item.getAttribute('data-context')).toBeNull();
      void c;
    });
    it('keyboard: ContextMenu key opens it with the first item focused; F2 renames a collection inline', async () => {
      const api = createMockApi();
      await api.createCollection('Dự án B', null);
      render(<Sidebar api={api} pickFolder={async () => null} />); await settle();
      const item = screen.getByRole('button', { name: /^Dự án B/ });
      item.focus(); fireEvent.keyDown(item, { key: 'F10', shiftKey: true }); await settle();
      expect(document.activeElement?.textContent).toMatch(/Mở/);
      fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' }); await settle();
      fireEvent.keyDown(item, { key: 'F2' }); await settle();
      expect(screen.getByRole('textbox', { name: 'Đổi tên dự án' })).toBeTruthy();
    });
    it('Gần đây: clear history asks first; Đang bật: Tắt tất cả is disabled with a reason when nothing is active; plain items (Tất cả) have no menu', async () => {
      useUiStore.setState({ recentViews: [{ face: 'ab:0', at: 1 }] });
      await mount();
      fireEvent.contextMenu(screen.getByRole('button', { name: /Gần đây/ })); await settle();
      fireEvent.click(screen.getByRole('menuitem', { name: /Xoá lịch sử/ })); await settle();
      expect(useUiStore.getState().recentViews).toHaveLength(1);
      fireEvent.click(screen.getByTestId('smenu-cancel')); await settle();
      expect(useUiStore.getState().recentViews).toHaveLength(1);
      fireEvent.contextMenu(screen.getByRole('button', { name: /Gần đây/ })); await settle();
      fireEvent.click(screen.getByRole('menuitem', { name: /Xoá lịch sử/ })); await settle();
      fireEvent.click(screen.getByTestId('smenu-confirm')); await settle();
      expect(useUiStore.getState().recentViews).toHaveLength(0);
      fireEvent.contextMenu(screen.getByRole('button', { name: /Đang bật/ })); await settle();
      expect(screen.getByRole('menuitem', { name: /Tắt tất cả/ }).getAttribute('aria-disabled')).toBe('true');
      fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' }); await settle();
      const prevented = !fireEvent.contextMenu(screen.getByRole('button', { name: /Tất cả/ }));
      expect(prevented).toBe(false); // không chặn: menu hệ thống vẫn như cũ
      expect(screen.queryByRole('menu')).toBeNull();
    });
    it('tag: Lọc theo thẻ này sets the tag', async () => {
      const { api } = await mount();
      const t = await api.createTag('serif');
      useUiStore.getState().bumpLibrary(false); await settle();
      fireEvent.contextMenu(await screen.findByRole('button', { name: 'serif' })); await settle();
      fireEvent.click(screen.getByRole('menuitem', { name: 'Lọc theo thẻ này' })); await settle();
      expect(useUiStore.getState().filters.tagIds).toEqual([t.id]);
    });
  });

  describe('Dự án: Bật tất cả (R-activate)', () => {
    it('menu has "Bật tất cả (N)" before Tắt tất cả; it activates the collection through the activation API', async () => {
      const { api } = await mount();
      const col = await api.createCollection('Dự án B', null);
      const rows = (await api.queryFaces({ limit: 3 })).items;
      await api.addToCollection(col.id, rows.map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })));
      useUiStore.getState().bumpLibrary(false); await settle();
      const spy = vi.spyOn(api, 'activationActivateCollection');
      fireEvent.contextMenu(screen.getByRole('button', { name: /^Dự án B/ })); await settle();
      const items = screen.getAllByRole('menuitem').map(i => i.textContent ?? '');
      expect(items.findIndex(x => /Bật tất cả \(3\)/.test(x))).toBeGreaterThan(-1);
      expect(items.findIndex(x => /Bật tất cả/.test(x))).toBeLessThan(items.findIndex(x => /Tắt tất cả/.test(x)));
      fireEvent.click(screen.getByRole('menuitem', { name: /Bật tất cả/ })); await settle();
      expect(spy).toHaveBeenCalledWith(col.id);
    });
    it('a double click on Bật tất cả sends one request while the first is running', async () => {
      const { api } = await mount();
      const col = await api.createCollection('Kép', null);
      const rows = (await api.queryFaces({ limit: 2 })).items;
      await api.addToCollection(col.id, rows.map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })));
      useUiStore.getState().bumpLibrary(false); await settle();
      let release: () => void = () => {};
      const real = api.activationActivateCollection.bind(api);
      const spy = vi.spyOn(api, 'activationActivateCollection').mockImplementation(async (id, o) => { await new Promise<void>(r => { release = r; }); return real(id, o); });
      for (let n = 0; n < 2; n++) {
        fireEvent.contextMenu(screen.getByRole('button', { name: /^Kép/ })); await settle();
        fireEvent.click(screen.getByRole('menuitem', { name: /Bật tất cả/ })); await settle();
      }
      expect(spy).toHaveBeenCalledTimes(1);
      await act(async () => { release(); }); await settle();
    });
    it('an empty project: the entry is disabled with a reason', async () => {
      const { api } = await mount();
      await api.createCollection('Rỗng', null); useUiStore.getState().bumpLibrary(false); await settle();
      fireEvent.contextMenu(screen.getByRole('button', { name: /^Rỗng/ })); await settle();
      expect(screen.getByRole('menuitem', { name: /Bật tất cả/ }).getAttribute('aria-disabled')).toBe('true');
    });
  });

  describe('Gỡ nguồn (R7)', () => {
    it('source menu → Gỡ nguồn… opens the preview with real numbers; confirm removes it, leaves its view, and tells how many locations went', async () => {
      const { api } = await mount();
      const target = (await api.listSources()).find(s => s.name === 'Dự án khách')!;
      useUiStore.getState().setView(`src:${target.id}` as never); await settle();
      fireEvent.contextMenu(screen.getByRole('button', { name: /Dự án khách/ })); await settle();
      fireEvent.click(screen.getByRole('menuitem', { name: /Gỡ nguồn/ })); await settle();
      const preview = await api.removeSourcePreview(target.id);
      expect(screen.getByTestId('rmsrc-facts').querySelector('[data-fact="locations"]')?.textContent).toContain(`${preview.removedLocations} vị trí`);
      expect((await api.listSources()).some(s => s.id === target.id)).toBe(true);
      fireEvent.click(screen.getByTestId('rmsrc-confirm')); await settle();
      expect((await api.listSources()).some(s => s.id === target.id)).toBe(false);
      expect(useUiStore.getState().view).toBe('all');
      expect(screen.queryByRole('button', { name: /Dự án khách/ })).toBeNull();
    });
    it('is disabled with a reason while a scan runs', async () => {
      await mount();
      useScanStore.setState({ scanning: true });
      fireEvent.contextMenu(screen.getByRole('button', { name: /Dự án khách/ })); await settle();
      expect(screen.getByRole('menuitem', { name: /Gỡ nguồn/ }).getAttribute('aria-disabled')).toBe('true');
      useScanStore.setState({ scanning: false });
    });
  });

  describe('Thẻ: đổi tên và xoá (R6)', () => {
    const openMenu = async (name: string | RegExp) => { fireEvent.contextMenu(await screen.findByRole('button', { name })); await settle(); };
    const seed = async () => { const m = await mount(); const a = await m.api.createTag('serif'); await m.api.createTag('display'); useUiStore.getState().bumpLibrary(false); await settle(); return { ...m, a }; };
    it('menu lists Lọc / Đổi tên (F2) / Xoá thẻ…; Đổi tên edits inline, Enter saves, focus returns to the chip, the name changes', async () => {
      const { api, a } = await seed();
      await openMenu('serif');
      const items = screen.getAllByRole('menuitem').map(i => i.textContent ?? '');
      expect(items).toHaveLength(3);
      expect(items[0]).toMatch(/Lọc theo thẻ này/); expect(items[1]).toMatch(/Đổi tên.*F2/); expect(items[2]).toMatch(/Xoá thẻ…/);
      fireEvent.click(screen.getByRole('menuitem', { name: /Đổi tên/ })); await settle();
      const box = screen.getByTestId('tag-rename') as HTMLInputElement;
      expect(box.value).toBe('serif');
      fireEvent.change(box, { target: { value: 'serifs' } });
      fireEvent.keyDown(box, { key: 'Enter' }); await settle();
      expect(screen.queryByTestId('tag-rename')).toBeNull();
      expect((await api.listTags()).find(t => t.id === a.id)?.name).toBe('serifs');
      expect(document.activeElement?.textContent).toBe('serifs');
    });
    it('F2 on a focused chip starts the rename; Esc cancels and keeps the name', async () => {
      await seed();
      const chip = screen.getByRole('button', { name: 'display' });
      chip.focus(); fireEvent.keyDown(chip, { key: 'F2' }); await settle();
      const box = screen.getByTestId('tag-rename');
      fireEvent.change(box, { target: { value: 'zzz' } });
      fireEvent.keyDown(box, { key: 'Escape' }); await settle();
      expect(screen.getByRole('button', { name: 'display' })).toBeTruthy();
      expect(document.activeElement).toBe(screen.getByRole('button', { name: 'display' }));
    });
    it('a duplicate name is reported (no silent merge) and the box keeps what was typed; empty is reported too', async () => {
      const { api } = await seed();
      await openMenu('serif');
      fireEvent.click(screen.getByRole('menuitem', { name: /Đổi tên/ })); await settle();
      const box = screen.getByTestId('tag-rename') as HTMLInputElement;
      fireEvent.change(box, { target: { value: 'Display' } });
      fireEvent.keyDown(box, { key: 'Enter' }); await settle();
      expect(screen.getByText(/Đã có thẻ khác tên «Display»/)).toBeTruthy();
      expect((screen.getByTestId('tag-rename') as HTMLInputElement).value).toBe('Display');
      expect((await api.listTags()).map(t => t.name).sort()).toEqual(['display', 'serif']);
      fireEvent.change(screen.getByTestId('tag-rename'), { target: { value: '  ' } });
      fireEvent.keyDown(screen.getByTestId('tag-rename'), { key: 'Enter' }); await settle();
      expect(screen.getByText('Tên thẻ không được để trống.')).toBeTruthy();
    });
    it('NFC: the decomposed spelling of the same name is not a rename (no API call); a real rename is sent in NFC', async () => {
      const { api } = await seed();
      await api.createTag('Phở'); useUiStore.getState().bumpLibrary(false); await settle();
      const spy = vi.spyOn(api, 'renameTag');
      await openMenu('Phở');
      fireEvent.click(screen.getByRole('menuitem', { name: /Đổi tên/ })); await settle();
      fireEvent.change(screen.getByTestId('tag-rename'), { target: { value: 'Pho\u031b\u0309' } }); // NFD của "Phở"
      fireEvent.keyDown(screen.getByTestId('tag-rename'), { key: 'Enter' }); await settle();
      expect(spy).not.toHaveBeenCalled();
      await openMenu('Phở');
      fireEvent.click(screen.getByRole('menuitem', { name: /Đổi tên/ })); await settle();
      fireEvent.change(screen.getByTestId('tag-rename'), { target: { value: 'Cafe\u0301' } }); // NFD của "Café"
      fireEvent.keyDown(screen.getByTestId('tag-rename'), { key: 'Enter' }); await settle();
      expect(spy).toHaveBeenCalledTimes(1);
      expect(spy.mock.calls[0][1]).toBe('Café');
    });
    it('system tags (style:/mood:/use:) have no Đổi tên / Xoá and F2 does nothing', async () => {
      const { api } = await seed();
      const sysName = (await api.systemTagCatalog())[0].tags[0].name;
      await api.createTag(sysName); useUiStore.getState().bumpLibrary(false); await settle();
      await openMenu(sysName);
      expect(screen.getAllByRole('menuitem').map(i => i.textContent)).toEqual([expect.stringMatching(/Lọc theo thẻ này/)]);
      fireEvent.keyDown(screen.getByRole('menu'), { key: 'Escape' }); await settle();
      const chip = screen.getByRole('button', { name: sysName });
      chip.focus(); fireEvent.keyDown(chip, { key: 'F2' }); await settle();
      expect(screen.queryByTestId('tag-rename')).toBeNull();
    });
    it('Xoá thẻ… asks first, names how many fonts lose the tag, and cancel keeps it; confirm deletes and clears the tag filter', async () => {
      const { api, a } = await seed();
      const rows = (await api.queryFaces({ limit: 3 })).items;
      await api.tagFaces(a.id, rows.map(r => ({ hashHex: r.hashHex, faceIndex: r.faceIndex })));
      useUiStore.getState().bumpLibrary(false); await settle();
      useUiStore.getState().setTag(String(a.id)); await settle();
      await openMenu('serif');
      fireEvent.click(screen.getByRole('menuitem', { name: /Xoá thẻ/ })); await settle();
      expect(screen.getByText(/Thẻ bị bỏ khỏi 3 font/)).toBeTruthy();
      fireEvent.click(screen.getByTestId('smenu-cancel')); await settle();
      expect((await api.listTags()).some(t => t.id === a.id)).toBe(true);
      await openMenu('serif');
      fireEvent.click(screen.getByRole('menuitem', { name: /Xoá thẻ/ })); await settle();
      fireEvent.click(screen.getByTestId('smenu-confirm')); await settle();
      expect((await api.listTags()).some(t => t.id === a.id)).toBe(false);
      expect(screen.queryByRole('button', { name: 'serif' })).toBeNull();
      expect(useUiStore.getState().filters.tagIds).toBeUndefined();
    });
  });

  describe('review fixes', () => {
    const report = (o: Partial<import('../../../bindings').ActivationReportDto>) => ({ results: [], changed: 0, needsConfirmation: 0, failed: 0, ...o });
    it('Tắt tất cả reports failures and needs-confirmation instead of staying silent', async () => {
      const { api } = await mount();
      vi.spyOn(api, 'activationDeactivateAll').mockResolvedValue(report({ changed: 2, failed: 3, needsConfirmation: 1 }));
      useNoticeStore.setState({ notices: [] });
      useUiStore.setState({ active: new Map([['aa', [0]]]) });
      fireEvent.contextMenu(screen.getByRole('button', { name: /Đang bật/ })); await settle();
      fireEvent.click(screen.getByRole('menuitem', { name: /Tắt tất cả/ })); await settle();
      const text = useNoticeStore.getState().notices.map(n => n.text).join(' | ');
      expect(text).toContain('Đã tắt 2 font, 3 lỗi');
      expect(text).toContain('1 font cần bạn xác nhận');
      expect(useNoticeStore.getState().notices[0].kind).toBe('error');
    });
    it('collection menu counts ACTIVE fonts: disabled with a reason when none are on', async () => {
      const api = createMockApi();
      const c = await api.createCollection('Dự án C', null);
      render(<Sidebar api={api} pickFolder={async () => null} />); await settle();
      fireEvent.contextMenu(screen.getByRole('button', { name: /^Dự án C/ })); await settle(); await settle();
      const item = screen.getByRole('menuitem', { name: /Tắt tất cả/ });
      expect(item.getAttribute('aria-disabled')).toBe('true');
      void c;
    });
    it('rename: an API error keeps the box with the typed name and shows the error; empty is reported, not silent; success returns focus to the item', async () => {
      const api = createMockApi();
      await api.createCollection('Cũ', null);
      render(<Sidebar api={api} pickFolder={async () => null} />); await settle();
      const item = screen.getByRole('button', { name: /^Cũ/ });
      fireEvent.keyDown(item, { key: 'F2' }); await settle();
      let box = screen.getByRole('textbox', { name: 'Đổi tên dự án' }) as HTMLInputElement;
      useNoticeStore.setState({ notices: [] });
      vi.spyOn(api, 'renameCollection').mockRejectedValueOnce(new Error('db locked'));
      fireEvent.change(box, { target: { value: 'Mới' } });
      box.value = 'Mới'; fireEvent.keyDown(box, { key: 'Enter' }); await settle();
      box = screen.getByRole('textbox', { name: 'Đổi tên dự án' }) as HTMLInputElement; // vẫn còn
      expect(box.value).toBe('Mới');
      expect(screen.getByRole('status').textContent).toContain('db locked');
      box.value = '   '; fireEvent.keyDown(box, { key: 'Enter' }); await settle();
      expect(screen.getByRole('status').textContent).toContain('không được để trống');
      box.value = 'Mới'; fireEvent.keyDown(box, { key: 'Enter' }); await settle(); await settle();
      expect(screen.queryByRole('textbox', { name: 'Đổi tên dự án' })).toBeNull();
      expect(screen.getByRole('button', { name: /^Mới/ })).toBeTruthy();
      await act(async () => { await new Promise(r => setTimeout(r, 30)); });
      expect(document.activeElement).toBe(screen.getByRole('button', { name: /^Mới/ }));
    });
    it('a tag id that no longer exists is dropped from the filters when the tags load', async () => {
      useUiStore.setState({ filters: { tagIds: [9999] } });
      await mount(); await settle();
      expect(useUiStore.getState().filters.tagIds).toBeUndefined();
    });
  });
});

describe('Sidebar: đích thả font (G1)', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useFaceDragStore.setState({ drag: null }); });
  it('mỗi dự án là đích thả; dự án đang xem đánh dấu "here"; rê qua khi đang kéo thì sáng lên (hợp lệ) hoặc mờ (đang xem)', async () => {
    const api = createMockApi(); const a = await api.createCollection('Menu', null); const b = await api.createCollection('Poster', null);
    useUiStore.setState({ view: `col:${b.id}` as never });
    const { container } = render(<Sidebar api={api} pickFolder={async () => null} />); await settle();
    const wrapA = container.querySelector(`[data-drop-col="${a.id}"]`) as HTMLElement; const wrapB = container.querySelector(`[data-drop-col="${b.id}"]`) as HTMLElement;
    expect(wrapA.dataset.dropName).toBe('Menu'); expect(wrapA.dataset.dropHere).toBeUndefined(); expect(wrapB.dataset.dropHere).toBe('true');
    expect(wrapA.dataset.dropOver).toBeUndefined();
    const rows = (await api.queryFaces({ limit: 2 })).items as never[];
    act(() => useFaceDragStore.setState({ drag: { rows, x: 1, y: 1, over: { kind: 'col', id: a.id, name: 'Menu', valid: true } } }));
    expect(wrapA.dataset.dropOver).toBe('ok'); expect(wrapA.dataset.dropArmed).toBe('true'); expect(wrapB.dataset.dropArmed).toBeUndefined();
    act(() => useFaceDragStore.setState({ drag: { rows, x: 1, y: 1, over: { kind: 'col', id: b.id, name: 'Poster', valid: false } } }));
    expect(wrapB.dataset.dropOver).toBe('no'); expect(wrapA.dataset.dropOver).toBeUndefined();
  });
});

describe('Sidebar: nhóm Case (G1)', () => {
  beforeEach(() => { useUiStore.setState(initial, true); useFaceDragStore.setState({ drag: null }); });
  it('không có Case thì không có nhóm; có Case thì liệt kê, là đích thả, bấm mở Case trong Pairing Desk ở Combo chính', async () => {
    const api = createMockApi();
    const none = render(<Sidebar api={api} pickFolder={async () => null} />); await settle();
    expect(none.container.querySelector('[data-drop-case]')).toBeNull(); none.unmount();
    const [r] = (await api.queryFaces({ limit: 1 })).items;
    const res = await api.openCaseFromPeek({ face: { hashHex: r.hashHex, faceIndex: r.faceIndex }, as: 'heading', candidate: null, caseId: null, settings: { vi: true, mode: 'safe' }, replace: false });
    const { container } = render(<Sidebar api={api} pickFolder={async () => null} />); await settle();
    const item = container.querySelector(`[data-drop-case="${res.case.id}"]`) as HTMLElement;
    expect(item).toBeTruthy(); expect(item.dataset.dropName).toBe(res.case.name);
    const rows = (await api.queryFaces({ limit: 2 })).items as never[];
    act(() => useFaceDragStore.setState({ drag: { rows, x: 1, y: 1, over: { kind: 'case', id: res.case.id, name: res.case.name, valid: true } } }));
    expect(item.dataset.dropOver).toBe('ok');
    act(() => useFaceDragStore.setState({ drag: null }));
    fireEvent.click(item.querySelector('button')!); await settle();
    expect(useUiStore.getState().desk?.caseId).toBe(res.case.id);
  });
});
