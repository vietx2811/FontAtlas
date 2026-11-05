import { act, fireEvent, render, screen, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { CmdError } from '../../../api';
import { Onboarding } from '../Onboarding';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const click = async (name: string | RegExp) => { await act(async () => { fireEvent.click(screen.getByRole('button', { name })); }); await settle(); };

async function mount(opts: Parameters<typeof createMockApi>[0] = { firstRun: true }, extra: Partial<React.ComponentProps<typeof Onboarding>> = {}) {
  const api = createMockApi(opts);
  const onDone = vi.fn();
  const step = vi.spyOn(api, 'setOnboardingStep');
  render(<Onboarding api={api} pickFolder={async () => '/Users/me/FontsDemo'} onDone={onDone} {...extra} />);
  await settle();
  return { api, onDone, step };
}
const heading = () => screen.getByRole('heading', { level: 1 }).textContent;

describe('Onboarding', () => {
  beforeEach(() => document.documentElement.removeAttribute('data-theme'));
  afterEach(() => vi.useRealTimers());

  it('walks the whole first run and saves every step', async () => {
    const { api, onDone, step } = await mount();
    expect(heading()).toBe('Chào mừng đến với Font Atlas');
    expect(screen.getByRole('group', { name: /Bước 1/ })).toBeTruthy();
    await click('Bắt đầu');
    expect(heading()).toBe('Thư mục font');
    expect(step).toHaveBeenLastCalledWith('o3'); // không có dữ liệu cũ → bỏ qua O2
    expect((screen.getByRole('button', { name: 'Tiếp tục' }) as HTMLButtonElement).disabled).toBe(true);
    await click(/Thêm thư mục/);
    expect(screen.getByText('/Users/me/FontsDemo')).toBeTruthy();
    expect((await api.listSources()).some(s => s.path === '/Users/me/FontsDemo')).toBe(true);
    await click('Tiếp tục');
    expect(heading()).toBe('Đang quét thư viện');
    await click('Tiếp tục');
    expect(heading()).toBe('Giấy phép và trách nhiệm');
    await click('Đã hiểu');
    expect((await api.getSettings()).licenseNoticeAck).toBe(true);
    expect(heading()).toBe('Phân tích hình ảnh (tuỳ chọn)');
    await click('Để sau');
    expect(heading()).toBe('Kích hoạt font cho app thiết kế');
    await click('Để sau');
    expect(heading()).toBe('Xong');
    await click('Vào thư viện');
    expect(step).toHaveBeenLastCalledWith('done');
    expect(onDone).toHaveBeenCalledTimes(1);
  });

  it('resumes where it stopped', async () => {
    const api = createMockApi({ firstRun: true });
    await api.setOnboardingStep('o5');
    render(<Onboarding api={api} onDone={() => {}} />);
    await settle();
    expect(heading()).toBe('Giấy phép và trách nhiệm');
  });

  it('"Bỏ qua hướng dẫn" ends it at once (marked done, not forever pending)', async () => {
    const { onDone, step } = await mount();
    await click('Bỏ qua hướng dẫn');
    expect(step).toHaveBeenLastCalledWith('done');
    expect(onDone).toHaveBeenCalled();
  });

  it('language and theme apply immediately', async () => {
    const { api } = await mount();
    fireEvent.click(screen.getByRole('radio', { name: 'English' }));
    await settle();
    expect((await api.getSettings()).uiLang).toBe('en');
    fireEvent.click(screen.getByRole('radio', { name: 'Tối' }));
    expect(document.documentElement.dataset.theme).toBe('dark');
    fireEvent.click(screen.getByRole('radio', { name: 'Theo hệ thống' }));
    expect(document.documentElement.dataset.theme).toBeUndefined();
  });

  it('O2 shows what was found in the old version and imports it on request', async () => {
    const { api } = await mount({ firstRun: true, legacy: true });
    await click('Bắt đầu');
    expect(heading()).toBe('Dữ liệu từ phiên bản trước');
    expect(screen.getByText(/2 nguồn · 17 yêu thích · 3 collection · 9 thẻ/)).toBeTruthy();
    const spy = vi.spyOn(api, 'legacyImport');
    await click('Chuyển dữ liệu');
    expect(spy).toHaveBeenCalled();
    expect(screen.getByText(/Đã chuyển/)).toBeTruthy();
  });

  it('O5 cannot be skipped: no "Để sau", but you can go back', async () => {
    const api = createMockApi({ firstRun: true });
    await api.setOnboardingStep('o5');
    render(<Onboarding api={api} onDone={() => {}} />);
    await settle();
    expect(screen.queryByRole('button', { name: 'Để sau' })).toBeNull();
    expect(screen.getByText(/Dữ liệu phân tích và hình ảnh không bao giờ rời khỏi máy/)).toBeTruthy();
    await click('Quay lại');
    expect(heading()).toBe('Thư mục font');
  });

  it('O6: disk space check, "Không dùng" turns analysis off, "Tải ngay" starts the download and moves on', async () => {
    const api = createMockApi({ firstRun: true, initial: { model: { diskFree: 100 * 1024 * 1024 } } });
    await api.setOnboardingStep('o6');
    render(<Onboarding api={api} onDone={() => {}} />);
    await settle();
    expect((screen.getByRole('button', { name: 'Tải ngay' }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText(/Cần thêm \d+ MB trống/)).toBeTruthy();
    cleanupAll();
    const api2 = createMockApi({ firstRun: true });
    await api2.setOnboardingStep('o6');
    const dl = vi.spyOn(api2, 'modelDownload');
    render(<Onboarding api={api2} onDone={() => {}} />);
    await settle();
    expect(screen.getByText(/371 MB/)).toBeTruthy();
    await click('Tải ngay');
    expect((await api2.getSettings()).embedEnabled).toBe(true);
    expect(dl).toHaveBeenCalled();
    expect(heading()).toBe('Kích hoạt font cho app thiết kế');
  });

  it('O6 "Không dùng" sets embedEnabled=false', async () => {
    const api = createMockApi({ firstRun: true });
    await api.setSettings({ embedEnabled: true });
    await api.setOnboardingStep('o6');
    render(<Onboarding api={api} onDone={() => {}} />);
    await settle();
    await click('Không dùng');
    expect((await api.getSettings()).embedEnabled).toBe(false);
    expect(heading()).toBe('Kích hoạt font cho app thiết kế');
  });

  it('O7: default is "turn off with the app"; the choice is saved with activation_set_keep', async () => {
    const api = createMockApi({ firstRun: true });
    await api.setOnboardingStep('o7');
    const keep = vi.spyOn(api, 'activationSetKeep');
    render(<Onboarding api={api} onDone={() => {}} />);
    await settle();
    expect((screen.getByRole('radio', { name: /Tắt luôn các font đã kích hoạt/ }) as HTMLInputElement).checked).toBe(true);
    await click('Bật kích hoạt font');
    expect(keep).toHaveBeenLastCalledWith(false);
    expect(heading()).toBe('Xong');
  });

  it('O7 "keep active" maps to activation_set_keep(true); the real activation is never triggered here', async () => {
    const api = createMockApi({ firstRun: true });
    await api.setOnboardingStep('o7');
    const keep = vi.spyOn(api, 'activationSetKeep');
    const activate = vi.spyOn(api, 'activationActivate');
    render(<Onboarding api={api} onDone={() => {}} />);
    await settle();
    fireEvent.click(screen.getByRole('radio', { name: /Giữ font hoạt động/ }));
    await click('Bật kích hoạt font');
    expect(keep).toHaveBeenLastCalledWith(true);
    expect(activate).not.toHaveBeenCalled();
  });

  it('O7 mentions another font manager only when one is installed', async () => {
    const api = createMockApi({ firstRun: true });
    await api.setOnboardingStep('o7');
    render(<Onboarding api={api} onDone={() => {}} />);
    await settle();
    expect(screen.getByText(/Phát hiện FontBase/)).toBeTruthy();
    expect(screen.queryByText(/Phát hiện RightFont/)).toBeNull();
  });

  it('O3 explains a nested folder instead of failing silently', async () => {
    const api = createMockApi({ firstRun: false });
    await api.setOnboardingStep('o3');
    render(<Onboarding api={api} pickFolder={async () => '/Users/me/Fonts/src1/sub'} onDone={() => {}} />);
    await settle();
    await click(/Thêm thư mục/);
    expect(screen.getByRole('alert').textContent).toMatch(/đã nằm trong nguồn/);
  });

  it('keeps O3 after legacy import and can add several independent folders', async () => {
    const api = createMockApi({ firstRun: false, legacy: true });
    await api.legacyImport();
    await api.setOnboardingStep('o1');
    render(<Onboarding api={api} pickFolders={async () => ['/Users/me/Thiết kế/Khách hàng A', '/Volumes/Font/Thư viện chữ Việt rất dài']} onDone={() => {}} />);
    await settle();
    await click('Bắt đầu');
    expect(heading()).toBe('Thư mục font');
    expect((screen.getByRole('button', { name: 'Tiếp tục' }) as HTMLButtonElement).disabled).toBe(false);
    const before = (await api.listSources()).length;
    await click(/Thêm thư mục/);
    expect((await api.listSources()).length).toBe(before + 2);
    expect(screen.getByText('/Volumes/Font/Thư viện chữ Việt rất dài')).toBeTruthy();
    await click('Tiếp tục');
    expect(heading()).toBe('Đang quét thư viện');
  });

  it('cancel is quiet; duplicate and containing sources keep the originals and continue the batch', async () => {
    const api = createMockApi();
    await api.setOnboardingStep('o3');
    const pickFolders = vi.fn().mockResolvedValueOnce([]).mockResolvedValueOnce(['/Users/me/Fonts/src1', '/Users/me/Fonts', '/Users/me/New']);
    render(<Onboarding api={api} pickFolders={pickFolders} onDone={() => {}} />);
    await settle();
    const old = (await api.listSources()).map(s => s.id);
    await click(/Thêm thư mục/);
    expect(screen.queryByRole('alert')).toBeNull();
    await click(/Thêm thư mục/);
    expect(screen.getByRole('alert').textContent).toContain('đã có trong thư viện');
    expect(screen.getByRole('alert').textContent).toContain('chứa nguồn đã có');
    expect((await api.listSources()).map(s => s.id)).toEqual(expect.arrayContaining(old));
    expect(screen.getByText('/Users/me/New')).toBeTruthy();
  });

  it('locks picker and navigation while pending, then reports permission failure without losing sources', async () => {
    const api = createMockApi();
    await api.setOnboardingStep('o3');
    let resolve!: (paths: string[]) => void;
    const pickFolders = vi.fn(() => new Promise<string[]>(r => { resolve = r; }));
    vi.spyOn(api, 'addSource').mockRejectedValue(new CmdError('io', 'Permission denied'));
    render(<Onboarding api={api} pickFolders={pickFolders} onDone={() => {}} />);
    await settle();
    await click(/Thêm thư mục/);
    await click(/Thêm thư mục/);
    expect(pickFolders).toHaveBeenCalledTimes(1);
    for (const name of ['Tiếp tục', 'Để sau', 'Quay lại']) expect((screen.getByRole('button', { name }) as HTMLButtonElement).disabled).toBe(true);
    await act(async () => resolve(['/Protected/Fonts']));
    await settle();
    expect(screen.getByRole('alert').textContent).toContain('Kiểm tra quyền đọc');
    expect((await api.listSources()).length).toBeGreaterThan(0);
    expect((screen.getByRole('button', { name: 'Tiếp tục' }) as HTMLButtonElement).disabled).toBe(false);
  });

  it('source load can retry; a busy deletion is shown and never removes the source', async () => {
    const api = createMockApi();
    await api.setOnboardingStep('o3');
    vi.spyOn(api, 'listSources').mockRejectedValueOnce(new Error('offline'));
    vi.spyOn(api, 'removeSource').mockRejectedValue(new CmdError('busy', 'scanning'));
    render(<Onboarding api={api} onDone={() => {}} />);
    await settle();
    expect(screen.getByRole('alert').textContent).toContain('offline');
    await click('Thử lại');
    expect(screen.queryByRole('alert')).toBeNull();
    const source = (await api.listSources())[0];
    await click(`Xoá ${source.name}`);
    expect(screen.getByRole('alert').textContent).toContain('Nguồn đang quét');
    expect((await api.listSources()).some(s => s.id === source.id)).toBe(true);
  });

  it('O4 counts the scan as it streams', async () => {
    vi.useFakeTimers();
    const api = createMockApi({ firstRun: true, tickMs: 100 });
    await api.setOnboardingStep('o3');
    render(<Onboarding api={api} pickFolder={async () => '/Users/me/NewFonts'} onDone={() => {}} />);
    await act(async () => { await vi.advanceTimersByTimeAsync(10); });
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: /Thêm thư mục/ })); await vi.advanceTimersByTimeAsync(10); });
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Tiếp tục' })); await vi.advanceTimersByTimeAsync(10); });
    expect(heading()).toBe('Đang quét thư viện');
    await act(async () => { await vi.advanceTimersByTimeAsync(600); });
    expect(screen.getByTestId('scan-summary').textContent).toMatch(/Đã tìm 200 file · đã phân tích 195 font · 3 file có vấn đề/);
    expect(screen.getByRole('img', { name: 'Đã quét xong' })).toBeTruthy(); // xong → vòng tròn xanh
    expect(screen.queryByRole('progressbar')).toBeNull();
    expect(screen.getAllByTestId('scan-block')).toHaveLength(9);
  });
});

// nhỏ gọn: gỡ mọi thứ đã render giữa hai lần dựng trong cùng một test
import { cleanup } from '@testing-library/react';
const cleanupAll = () => cleanup();
void within;
