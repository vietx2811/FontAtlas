import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { useScanStore } from '../../../store/scan';
import { useUiStore } from '../../../store/ui';
import { Sidebar, SidebarToggle } from '../Sidebar';

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
    expect(gone.getAttribute('title')).toMatch(/chưa kết nối/i);
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
    fireEvent.click(screen.getByRole('menuitem', { name: 'Xoá dự án' }));
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
    expect(useUiStore.getState().tag).toBe(String(t.id));
    fireEvent.click(screen.getByRole('button', { name: 'serif' }));
    expect(useUiStore.getState().tag).toBeNull();
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
});
