import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { createModelStore } from '../../../store/model';
import { useScanStore } from '../../../store/scan';
import { useUiStore } from '../../../store/ui';
import { Settings, SETTINGS_GROUPS } from '../Settings';
import { tipOf } from '../../../test/tip';
import { setLocale, t } from '../../../i18n';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const initial = useUiStore.getState();

async function mount(over: Partial<React.ComponentProps<typeof Settings>> = {}, api = createMockApi()) {
  const onClose = vi.fn(); const onRerun = vi.fn();
  const store = createModelStore();
  await act(async () => { await store.getState().sync(api); });
  render(<Settings api={api} onClose={onClose} onRerunOnboarding={onRerun} modelStore={store} {...over} />);
  await settle();
  return { api, onClose, onRerun, store };
}
const go = async (name: string) => { await act(async () => { fireEvent.click(screen.getByRole('button', { name })); }); await settle(); };

describe('Settings', () => {
  beforeEach(() => { useUiStore.setState(initial, true); document.documentElement.removeAttribute('data-theme'); });

  it('has the groups of spec 5 in a left column and shows one group at a time', async () => {
    await mount();
    expect(SETTINGS_GROUPS.map(g => g.label)).toEqual(['Chung', 'Thư viện và nguồn', 'Phân tích hình ảnh', 'Kích hoạt font', 'Bật font tự động', 'Giấy phép', 'Dữ liệu', 'Phím tắt', 'Trợ giúp']);
    const nav = screen.getByRole('navigation', { name: 'Nhóm cài đặt' });
    expect(within(nav).getAllByRole('button')).toHaveLength(9);
    expect(within(nav).getByRole('button', { name: 'Chung' }).getAttribute('aria-current')).toBe('true');
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('Chung');
    await go('Giấy phép');
    expect(screen.getByRole('heading', { level: 2 }).textContent).toBe('Giấy phép');
  });

  it('Esc and the close button leave the screen', async () => {
    const { onClose } = await mount();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole('button', { name: 'Xong' }));
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  describe('Chung', () => {
    it('language, theme and the default sample apply at once', async () => {
      const { api } = await mount();
      const set = vi.spyOn(api, 'setSettings');
      fireEvent.click(screen.getByRole('radio', { name: 'English' }));
      await settle();
      expect(set).toHaveBeenLastCalledWith({ uiLang: 'en' });
      expect(document.documentElement.lang).toBe('en');
      // Ngôn ngữ đổi ngay, không cần khởi động lại: nhãn đã sang tiếng Anh.
      fireEvent.click(screen.getByRole('radio', { name: 'Dark' }));
      expect(document.documentElement.dataset.theme).toBe('dark');
      fireEvent.click(screen.getByRole('radio', { name: 'Tiếng Việt' }));
      await settle();
      expect(set).toHaveBeenLastCalledWith({ uiLang: 'vi' });
      expect(screen.getByRole('radio', { name: 'Tối' })).toBeTruthy();
      fireEvent.change(screen.getByRole('textbox', { name: 'Chữ mẫu mặc định' }), { target: { value: 'Bún chả' } });
      await settle();
      expect(set).toHaveBeenLastCalledWith({ sampleText: 'Bún chả' });
      expect(useUiStore.getState().sample).toBe('Bún chả');
    });

    it('presets fill the sample; "Dự án dùng tiếng Việt" is a saved switch', async () => {
      const { api } = await mount();
      const set = vi.spyOn(api, 'setSettings');
      fireEvent.click(screen.getByRole('button', { name: 'Thực đơn có giá' }));
      await settle();
      expect(useUiStore.getState().sample).toMatch(/35\.000/);
      fireEvent.click(screen.getByRole('switch', { name: 'Dự án dùng tiếng Việt mặc định' }));
      await settle();
      expect(set).toHaveBeenLastCalledWith({ projectViDefault: true });
    });
  });

  describe('Thư viện và nguồn', () => {
    it('lists sources with count and state; an unplugged drive says so', async () => {
      await mount();
      await go('Thư viện và nguồn');
      const rows = screen.getAllByTestId('source-row');
      expect(rows).toHaveLength(3);
      expect(rows[0].textContent).toMatch(/Fonts/);
      expect(rows[0].textContent).toMatch(/Sẵn sàng/);
      expect(rows[2].textContent).toMatch(/Không khả dụng/);
    });

    it('rescan one source, remove after a confirmation that says files are kept', async () => {
      const { api } = await mount();
      await go('Thư viện và nguồn');
      const rescan = vi.spyOn(api, 'rescanSources');
      const rows = screen.getAllByTestId('source-row');
      fireEvent.click(within(rows[1]).getByRole('button', { name: 'Quét lại' }));
      expect(rescan).toHaveBeenCalledWith([2]);
      fireEvent.click(within(rows[1]).getByRole('button', { name: 'Xoá nguồn' }));
      const dlg = screen.getByRole('dialog');
      expect(dlg.textContent).toMatch(/không xoá file/i);
      fireEvent.click(within(dlg).getByRole('button', { name: 'Huỷ' }));
      expect(await api.listSources()).toHaveLength(3);
      fireEvent.click(within(screen.getAllByTestId('source-row')[1]).getByRole('button', { name: 'Xoá nguồn' }));
      await settle();
      expect(screen.getByTestId('rmsrc-facts')).toBeTruthy(); // số liệu thật từ bản xem trước
      await act(async () => { fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Gỡ khỏi thư viện' })); });
      await settle();
      expect((await api.listSources()).map(s => s.id)).toEqual([1, 3]);
      expect(screen.getAllByTestId('source-row')).toHaveLength(2);
    });

    it('shows how many files of a source are still in the cloud', async () => {
      await mount();
      await go('Thư viện và nguồn');
      expect(screen.getAllByTestId('source-row')[1].textContent).toMatch(/12 chưa tải về/);
      expect(screen.getAllByTestId('source-row')[0].textContent).not.toMatch(/chưa tải về/);
    });

    it('a source with files in the cloud has a Tải về máy button in its row; local and unplugged ones do not', async () => {
      const api = createMockApi();
      const dl = vi.spyOn(api, 'sourceDownloadCloud');
      await mount({}, api);
      await go('Thư viện và nguồn');
      const rows = screen.getAllByTestId('source-row');
      expect(within(rows[0]).queryByTestId('cloud-dl-start')).toBeNull();
      expect(within(rows[2]).queryByTestId('cloud-dl-start')).toBeNull();
      await act(async () => { fireEvent.click(within(rows[1]).getByTestId('cloud-dl-start')); });
      await settle();
      expect(dl).toHaveBeenCalledWith(2, expect.any(Function));
    });

    it('a cloud source shows the same hint as the sidebar; a local one does not', async () => {
      await mount();
      await go('Thư viện và nguồn');
      const rows = screen.getAllByTestId('source-row');
      expect(within(rows[1]).getByTestId('cloud-hint')).toBeTruthy();
      expect(await tipOf(within(rows[1]).getByTestId('cloud-hint'))).toMatch(/^90% font của nguồn này chưa tải về máy/);
      expect(within(rows[0]).queryByTestId('cloud-hint')).toBeNull();
    });

    it('"Tự phát hiện thay đổi trên ổ cloud" bật mặc định, lưu khi bấm, có lời giải thích; tiếng Anh cũng có', async () => {
      const { api } = await mount();
      await go('Thư viện và nguồn');
      const sw = screen.getByRole('switch', { name: 'Tự phát hiện thay đổi trên ổ cloud' });
      expect(sw.getAttribute('aria-checked')).toBe('true');
      expect(document.body.textContent).toMatch(/tự quét lại thư mục đó sau vài giây/);
      const set = vi.spyOn(api, 'setSettings');
      fireEvent.click(sw);
      await settle();
      expect(set).toHaveBeenLastCalledWith({ cloudWatch: false });
      expect(screen.getByRole('switch', { name: 'Tự phát hiện thay đổi trên ổ cloud' }).getAttribute('aria-checked')).toBe('false');
      setLocale('en');
      expect(t('cloud.watch')).toBe('Detect changes on cloud drives automatically');
      expect(t('cloud.watch.note')).toMatch(/rescans that folder/);
      setLocale('vi');
    });

    it('"Tự tải font trên cloud để xem trước" is on by default, is saved when switched, and explains dataless / Always keep', async () => {
      const { api } = await mount();
      await go('Thư viện và nguồn');
      const sw = screen.getByRole('switch', { name: 'Tự tải font trên cloud để xem trước' });
      expect(sw.getAttribute('aria-checked')).toBe('true');
      const set = vi.spyOn(api, 'setSettings');
      fireEvent.click(sw);
      await settle();
      expect(set).toHaveBeenLastCalledWith({ cloudPreviewDownload: false });
      expect(screen.getByRole('switch', { name: 'Tự tải font trên cloud để xem trước' }).getAttribute('aria-checked')).toBe('false');
      const dataless = document.querySelector('[data-term="dataless"]')!;
      expect(await tipOf(dataless)).toMatch(/bản giữ chỗ/);
      const keep = document.querySelector('[data-term="always_keep"]')!;
      const keepTip = await tipOf(keep);
      expect(keepTip).toMatch(/Always keep on this device/);
      expect(keepTip).toMatch(/Finder/);
    });

    it('a source that needs access says so and offers Cấp quyền and Thử lại', async () => {
      const { api } = await mount();
      await go('Thư viện và nguồn');
      act(() => useScanStore.setState({ access: { 2: 'timeout' } }));
      const row = screen.getAllByTestId('source-row')[1];
      expect(row.textContent).toMatch(/Cần quyền truy cập/);
      expect(within(row).getByRole('button', { name: 'Cấp quyền' })).toBeTruthy();
      const spy = vi.spyOn(api, 'rescanSources');
      fireEvent.click(within(row).getByRole('button', { name: 'Thử lại' }));
      expect(spy).toHaveBeenCalledWith([2]);
      act(() => useScanStore.setState({ access: {} }));
    });

    it('"Quét lại khi mở app" and "+ Thêm"', async () => {
      const { api } = await mount();
      await go('Thư viện và nguồn');
      const set = vi.spyOn(api, 'setSettings');
      fireEvent.click(screen.getByRole('switch', { name: 'Quét lại khi mở app' }));
      await settle();
      expect(set).toHaveBeenLastCalledWith({ rescanOnLaunch: false });
      const before = useUiStore.getState().addFolderRequest;
      fireEvent.click(screen.getByRole('button', { name: '+ Thêm' }));
      expect(useUiStore.getState().addFolderRequest).toBe(before + 1);
    });
  });

  describe('Phân tích hình ảnh', () => {
    it('the switch, the model line with its actions, and the queue progress', async () => {
      const api = createMockApi({ initial: { model: { state: 'ready' }, embed: { enabled: true, running: true, pending: 3000, done: 800, device: 'metal', etaMs: 120_000 } } });
      await api.setSettings({ embedEnabled: true });
      await mount({}, api);
      await go('Phân tích hình ảnh');
      expect(screen.getByRole('switch', { name: 'Bật phân tích hình ảnh' }).getAttribute('aria-checked')).toBe('true');
      expect(screen.getByTestId('model-line').textContent).toMatch(/Phân tích hình ảnh 800\/3\.800/);
      expect(screen.getByText(/Metal \(GPU\)/)).toBeTruthy();
      fireEvent.click(screen.getByRole('button', { name: 'Tạm dừng' }));
      await settle();
      expect(screen.getByTestId('model-line').textContent).toMatch(/tạm dừng/);
    });

    it('turning it off saves embedEnabled=false', async () => {
      const api = createMockApi();
      await api.setSettings({ embedEnabled: true });
      await mount({}, api);
      await go('Phân tích hình ảnh');
      const set = vi.spyOn(api, 'setSettings');
      fireEvent.click(screen.getByRole('switch', { name: 'Bật phân tích hình ảnh' }));
      await settle();
      expect(set).toHaveBeenLastCalledWith({ embedEnabled: false });
    });

    it('a missing model offers to download it, with the size', async () => {
      const api = createMockApi();
      await api.setSettings({ embedEnabled: true });
      await mount({}, api);
      await go('Phân tích hình ảnh');
      expect(screen.getByTestId('model-line').textContent).toMatch(/Chưa tải model/);
      expect(screen.getByText(/371 MB/)).toBeTruthy();
      await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Tải' })); });
      await settle();
      expect((await api.modelStatus()).state).toBe('downloading');
    });
  });

  describe('Kích hoạt font', () => {
    it('lists the old copies that were replaced and not yet restored, with a Restore button that removes the row', async () => {
      const api = createMockApi();
      const f = (await api.queryFaces({ text: 'Oswald', limit: 5 })).items[0];
      const key = { hashHex: f.hashHex, faceIndex: f.faceIndex };
      const pre = await api.activationPreflight([key]);
      const seen = pre.results.flatMap(r => (r.outcome.kind === 'needs_confirmation' ? r.outcome.conflicts.map(x => ({ hashHex: r.hashHex, existingPath: x.existingPath })) : []));
      await api.activationReplace([key], seen);
      await mount({}, api);
      await go('Kích hoạt font');
      const box = screen.getByTestId('replaced-list');
      expect(within(box).getByText(/Oswald\.ttf/)).toBeTruthy();
      await act(async () => { fireEvent.click(within(box).getByRole('button', { name: 'Khôi phục' })); });
      await settle();
      expect(within(screen.getByTestId('replaced-list')).queryByRole('button', { name: 'Khôi phục' })).toBeNull();
      expect(within(screen.getByTestId('replaced-list')).getByText('Chưa có bản cũ nào bị thay.')).toBeTruthy();
    });

    it('"when FontAtlas quits" is saved; other font managers are information only', async () => {
      const { api } = await mount();
      await go('Kích hoạt font');
      expect((screen.getByRole('radio', { name: /Tắt luôn/ }) as HTMLInputElement).checked).toBe(true);
      const set = vi.spyOn(api, 'setSettings');
      fireEvent.click(screen.getByRole('radio', { name: /Giữ font hoạt động/ }));
      await settle();
      expect(set).toHaveBeenLastCalledWith({ keepActiveOnExit: true });
      expect(screen.getByText(/FontBase/)).toBeTruthy();
    });

    it('turning every font off asks first, then deactivates all', async () => {
      const api = createMockApi();
      const [a] = (await api.queryFaces({ limit: 1 })).items;
      await api.activationActivate([{ hashHex: a.hashHex, faceIndex: 0 }]);
      await mount({}, api);
      await act(async () => { await useUiStore.getState().refreshActive(api); });
      await go('Kích hoạt font');
      const spy = vi.spyOn(api, 'activationDeactivateAll');
      expect(screen.getByRole('button', { name: /Tắt mọi font đã kích hoạt/ })).toBeTruthy();
      fireEvent.click(screen.getByRole('button', { name: /Tắt mọi font đã kích hoạt/ }));
      expect(spy).not.toHaveBeenCalled();
      const dlg = screen.getByRole('dialog');
      expect(dlg.textContent).toMatch(/1 font/);
      await act(async () => { fireEvent.click(within(dlg).getByRole('button', { name: 'Tắt tất cả' })); });
      await settle();
      expect(spy).toHaveBeenCalled();
      expect(await api.activationState()).toEqual([]);
    });
  });

  it('Giấy phép: the fixed responsibility notice', async () => {
    await mount();
    await go('Giấy phép');
    expect(screen.getByText(/Bạn tự chịu trách nhiệm tuân thủ giấy phép/)).toBeTruthy();
    expect(screen.getByText(/không gửi font hay kết quả đi đâu/)).toBeTruthy();
  });

  describe('Dữ liệu', () => {
    it('backup now shows where it went; cache size, clear it', async () => {
      const { api } = await mount();
      await go('Dữ liệu');
      await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Sao lưu ngay' })); });
      await settle();
      expect(screen.getByTestId('backup-path').textContent).toMatch(/\/user-.*\.db$/);
      expect(screen.getByTestId('cache-info').textContent).toMatch(/12 MB/);
      const spy = vi.spyOn(api, 'clearPreviewCache');
      await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Xoá bộ nhớ đệm' })); });
      await settle();
      expect(spy).toHaveBeenCalled();
      expect(screen.getByTestId('cache-info').textContent).toMatch(/0 MB|trống/);
    });

    it('lists fonts with problems with a retry; old-data import only when old data exists', async () => {
      const api = createMockApi({ legacy: true });
      vi.spyOn(api, 'listProblems').mockResolvedValue([{ hashHex: 'ab', faceIndex: 0, stage: 'analysis', code: 'truncated', detail: null, attempts: 2, path: '/f/x.ttf' }]);
      await mount({}, api);
      await go('Dữ liệu');
      expect(screen.getByTestId('problems').textContent).toMatch(/1 font có vấn đề/);
      expect(screen.getByTestId('problems').textContent).toMatch(/truncated/);
      const retry = vi.spyOn(api, 'retryProblems');
      await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Thử lại tất cả' })); });
      expect(retry).toHaveBeenCalled();
      expect(screen.getByRole('button', { name: 'Nhập dữ liệu cũ' })).toBeTruthy();
    });

    it('no old data: no import button', async () => {
      await mount();
      await go('Dữ liệu');
      expect(screen.queryByRole('button', { name: 'Nhập dữ liệu cũ' })).toBeNull();
    });
  });

  it('Phím tắt lists the shortcuts', async () => {
    await mount();
    await go('Phím tắt');
    expect(screen.getByText('⌘K')).toBeTruthy();
    expect(screen.getByText('⌘,')).toBeTruthy();
  });

  it('Trợ giúp: re-run the guide (restarts at step 1) and shows the model version', async () => {
    const { api, onRerun } = await mount({}, createMockApi({ initial: { model: { state: 'ready' }, embed: { enabled: true, modelVersion: 'siglip2' } } }));
    await go('Trợ giúp');
    expect(screen.getByText(/siglip2/)).toBeTruthy();
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Chạy lại hướng dẫn' })); });
    await settle();
    expect((await api.onboardingState()).step).toBe('o1');
    expect(onRerun).toHaveBeenCalled();
  });

  it('General has "Hiện splash khi mở app" (default on) that persists under ui.splash', async () => {
    const { api } = await mount();
    const sw = screen.getByRole('switch', { name: 'Hiện splash khi mở app' });
    expect(sw.getAttribute('aria-checked')).toBe('true');
    await act(async () => { fireEvent.click(sw); });
    await settle();
    expect(await api.getSetting('ui.splash')).toBe('off');
    expect(screen.getByRole('switch', { name: 'Hiện splash khi mở app' }).getAttribute('aria-checked')).toBe('false');
  });
});
