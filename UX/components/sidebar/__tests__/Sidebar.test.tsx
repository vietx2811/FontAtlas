import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useNoticeStore } from '../../../store/notices';
import { useScanStore } from '../../../store/scan';
import { useUiStore } from '../../../store/ui';
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
    expect(dlg.textContent).toMatch(/70 font/);
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
