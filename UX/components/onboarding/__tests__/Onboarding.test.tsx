import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { CmdError } from '../../../api';
import { Onboarding } from '../Onboarding';
import { setSplashOpen } from '../../splash/splashState';
import { setLocale } from '../../../i18n';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
// Chờ nút xuất hiện (máy bận: bước chưa kịp vẽ) rồi mới bấm.
const click = async (name: string | RegExp) => { const btn = await screen.findByRole('button', { name }); await act(async () => { fireEvent.click(btn); }); await settle(); };

async function mount(opts: Parameters<typeof createMockApi>[0] = { firstRun: true }, extra: Partial<React.ComponentProps<typeof Onboarding>> = {}) {
  const api = createMockApi(opts);
  const onDone = vi.fn();
  const step = vi.spyOn(api, 'setOnboardingStep');
  render(<Onboarding api={api} pickFolder={async () => '/Users/me/FontsDemo'} onDone={onDone} animated={false} {...extra} />);
  await settle();
  return { api, onDone, step };
}
const heading = () => screen.getByRole('heading', { level: 1 }).textContent;

describe('Onboarding', () => {
  beforeEach(() => document.documentElement.removeAttribute('data-theme'));
  afterEach(() => { vi.useRealTimers(); setLocale('vi'); });

  it('choosing English at O1 switches the title and numbers at once, and saves uiLang', async () => {
    const { api } = await mount();
    expect(heading()).toBe('Chào mừng đến với Font Atlas');
    await act(async () => { fireEvent.click(screen.getByRole('radio', { name: 'English' })); });
    await settle();
    expect(heading()).toBe('Welcome to Font Atlas');
    expect((await api.getSettings()).uiLang).toBe('en');
    await click('Get started');
    expect(heading()).not.toMatch(/[ăâđêôơư]/i);
  });

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
    await click('Vào thư viện'); await click('Bắt đầu');
    expect(step).toHaveBeenLastCalledWith('done');
    expect(onDone).toHaveBeenCalledTimes(1);
  });

  it('resumes where it stopped', async () => {
    const api = createMockApi({ firstRun: true });
    await api.setOnboardingStep('o5');
    render(<Onboarding api={api} onDone={() => {}} animated={false} />);
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
    expect(heading()).toBe('Welcome to Font Atlas');
    fireEvent.click(screen.getByRole('radio', { name: 'Dark' }));
    expect(document.documentElement.dataset.theme).toBe('dark');
    fireEvent.click(screen.getByRole('radio', { name: 'Match system' }));
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
    render(<Onboarding api={api} onDone={() => {}} animated={false} />);
    await settle();
    expect(screen.queryByRole('button', { name: 'Để sau' })).toBeNull();
    expect(screen.getByText(/Dữ liệu không rời khỏi máy/)).toBeTruthy();
    await click('Quay lại');
    expect(heading()).toBe('Thư mục font');
  });

  it('O6: disk space check, "Không dùng" turns analysis off, "Tải ngay" shows progress until the user continues', async () => {
    const api = createMockApi({ firstRun: true, initial: { model: { diskFree: 100 * 1024 * 1024 } } });
    await api.setOnboardingStep('o6');
    render(<Onboarding api={api} onDone={() => {}} animated={false} />);
    await settle();
    expect((screen.getByRole('button', { name: 'Tải ngay' }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText(/Cần thêm \d+ MB trống/)).toBeTruthy();
    cleanupAll();
    const api2 = createMockApi({ firstRun: true });
    await api2.setOnboardingStep('o6');
    const dl = vi.spyOn(api2, 'modelDownload');
    render(<Onboarding api={api2} onDone={() => {}} animated={false} />);
    await settle();
    expect(screen.getByText(/371 MB/)).toBeTruthy();
    await click('Tải ngay');
    expect((await api2.getSettings()).embedEnabled).toBe(true);
    expect(dl).toHaveBeenCalled();
    expect(heading()).toBe('Phân tích hình ảnh (tuỳ chọn)');
    expect(screen.getByText(/Đang tải model/)).toBeTruthy();
    await click('Tiếp tục');
    expect(heading()).toBe('Kích hoạt font cho app thiết kế');
  });

  it('O6 "Không dùng" sets embedEnabled=false', async () => {
    const api = createMockApi({ firstRun: true });
    await api.setSettings({ embedEnabled: true });
    await api.setOnboardingStep('o6');
    render(<Onboarding api={api} onDone={() => {}} animated={false} />);
    await settle();
    await click('Không dùng');
    expect((await api.getSettings()).embedEnabled).toBe(false);
    expect(heading()).toBe('Kích hoạt font cho app thiết kế');
  });

  it('O6 exposes checking and collapsed technical errors with cancellation resume', async () => {
    const api = createMockApi({ firstRun: true, initial: { model: { state: 'downloading', doneBytes: 100, totalBytes: 100 } } });
    await api.setOnboardingStep('o6');
    render(<Onboarding api={api} onDone={() => {}} animated={false} />); await settle();
    expect(await screen.findByText('Đang kiểm tra tệp…')).toBeTruthy(); // chờ theo điều kiện: model sync xong sau vài lượt khi máy bận
    expect(screen.getByTestId('model-spinner')).toBeTruthy();
    const { useModelStore } = await import('../../../store/model');
    act(() => useModelStore.setState(s => ({ model: { ...s.model!, state: 'paused', lastError: { code: 'tls', message: 'ONBOARD_TLS_DETAIL' } } })));
    expect(screen.getByRole('alert')).toHaveTextContent('Không xác thực');
    expect(screen.getByText('ONBOARD_TLS_DETAIL')).not.toBeVisible();
    act(() => useModelStore.setState(s => ({ model: { ...s.model!, lastError: { code: 'cancelled', message: 'user stop' } } })));
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.getByRole('button', { name: 'Tiếp tục tải' })).toBeTruthy();
  });

  it('O7: default is "turn off with the app"; the choice is saved with activation_set_keep', async () => {
    const api = createMockApi({ firstRun: true });
    await api.setOnboardingStep('o7');
    const keep = vi.spyOn(api, 'activationSetKeep');
    render(<Onboarding api={api} onDone={() => {}} animated={false} />);
    await settle();
    expect((screen.getByRole('radio', { name: /Tắt các font đã kích hoạt/ }) as HTMLInputElement).checked).toBe(true);
    await click('Bật kích hoạt font');
    expect(keep).toHaveBeenLastCalledWith(false);
    expect(heading()).toBe('Xong');
  });

  it('O7 "keep active" maps to activation_set_keep(true); the real activation is never triggered here', async () => {
    const api = createMockApi({ firstRun: true });
    await api.setOnboardingStep('o7');
    const keep = vi.spyOn(api, 'activationSetKeep');
    const activate = vi.spyOn(api, 'activationActivate');
    render(<Onboarding api={api} onDone={() => {}} animated={false} />);
    await settle();
    fireEvent.click(screen.getByRole('radio', { name: /Giữ font hoạt động/ }));
    await click('Bật kích hoạt font');
    expect(keep).toHaveBeenLastCalledWith(true);
    expect(activate).not.toHaveBeenCalled();
  });

  it('O7 mentions another font manager only when one is installed', async () => {
    const api = createMockApi({ firstRun: true });
    await api.setOnboardingStep('o7');
    render(<Onboarding api={api} onDone={() => {}} animated={false} />);
    await settle();
    expect(screen.getByText(/Phát hiện FontBase/)).toBeTruthy();
    expect(screen.queryByText(/Phát hiện RightFont/)).toBeNull();
  });

  it('O3 explains a nested folder instead of failing silently', async () => {
    const api = createMockApi({ firstRun: false });
    await api.setOnboardingStep('o3');
    render(<Onboarding api={api} pickFolder={async () => '/Users/me/Fonts/src1/sub'} onDone={() => {}} animated={false} />);
    await settle();
    await click(/Thêm thư mục/);
    expect(screen.getByRole('alert').textContent).toMatch(/đã nằm trong nguồn/);
  });

  it('keeps O3 after legacy import and can add several independent folders', async () => {
    const api = createMockApi({ firstRun: false, legacy: true });
    await api.legacyImport();
    await api.setOnboardingStep('o1');
    render(<Onboarding api={api} pickFolders={async () => ['/Users/me/Thiết kế/Khách hàng A', '/Volumes/Font/Thư viện chữ Việt rất dài']} onDone={() => {}} animated={false} />);
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
    render(<Onboarding api={api} pickFolders={pickFolders} onDone={() => {}} animated={false} />);
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
    render(<Onboarding api={api} pickFolders={pickFolders} onDone={() => {}} animated={false} />);
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
    render(<Onboarding api={api} onDone={() => {}} animated={false} />);
    await settle();
    expect(screen.getByRole('alert').textContent).toContain('offline');
    await click('Thử lại');
    expect(screen.queryByRole('alert')).toBeNull();
    const source = (await api.listSources())[0];
    await click(`Xoá ${source.name}`);
    expect(screen.getByRole('alert').textContent).toContain('Nguồn đang quét');
    expect((await api.listSources()).some(s => s.id === source.id)).toBe(true);
  });

  it('preserves per-folder failures when refreshing the source list also fails', async () => {
    const api = createMockApi(); await api.setOnboardingStep('o3');
    const sources = vi.spyOn(api, 'listSources');
    vi.spyOn(api, 'addSource').mockRejectedValueOnce(new CmdError('io', 'Folder A denied')).mockRejectedValueOnce(new CmdError('io', 'Folder B denied'));
    render(<Onboarding api={api} pickFolders={async () => ['/A', '/B']} onDone={() => {}} animated={false} />);
    await settle();
    sources.mockRejectedValueOnce(new Error('refresh unavailable'));
    await click(/Thêm thư mục/);
    const alerts = screen.getAllByRole('alert').map(e => e.textContent).join(' ');
    expect(alerts).toContain('Folder A denied'); expect(alerts).toContain('Folder B denied');
    expect(alerts).toContain('refresh unavailable');
  });

  it('does not delete current or queued sources from scan status', async () => {
    const api = createMockApi(); await api.setOnboardingStep('o3');
    const sources = await api.listSources();
    vi.spyOn(api, 'scanStatus').mockResolvedValue({ running: true, paused: false, currentSourceId: sources[0].id, queued: [sources[1].id] });
    const remove = vi.spyOn(api, 'removeSource');
    render(<Onboarding api={api} onDone={() => {}} animated={false} />); await settle();
    for (const source of sources.slice(0, 2)) {
      const button = screen.getByRole('button', { name: `Xoá ${source.name}` });
      expect(button).toHaveAttribute('aria-disabled', 'true'); fireEvent.click(button);
    }
    expect(remove).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Tiếp tục' })).not.toBeDisabled();
  });

  it('O4 counts the scan as it streams', async () => {
    vi.useFakeTimers();
    const api = createMockApi({ firstRun: true, tickMs: 100 });
    await api.setOnboardingStep('o3');
    render(<Onboarding api={api} pickFolder={async () => '/Users/me/NewFonts'} onDone={() => {}} animated={false} />);
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
  describe('lab layout, intro and outro', () => {
    const startFresh = async (extra: Partial<React.ComponentProps<typeof Onboarding>> = {}) => {
      const api = createMockApi({ firstRun: true });
      const onDone = vi.fn();
      render(<Onboarding api={api} onDone={onDone} {...extra} />);
      await settle();
      return { api, onDone };
    };

    it('reduced motion: "Vào thư viện" finishes at once, with no outro', async () => {
      vi.stubGlobal('matchMedia', (q: string) => ({ matches: q.includes('prefers-reduced-motion'), media: q, addEventListener: () => {}, removeEventListener: () => {} }));
      const api = createMockApi({ firstRun: true });
      await api.setOnboardingStep('end');
      const onDone = vi.fn();
      render(<Onboarding api={api} onDone={onDone} />);
      await settle();
      await click('Vào thư viện'); await click('Bắt đầu');
      expect(onDone).toHaveBeenCalledTimes(1);
      expect(screen.queryByTestId('onb-outro')).toBeNull();
      vi.unstubAllGlobals();
    });

    it('no intro of its own: the guide opens straight on step 1 (the splash belongs to the app)', async () => {
      await startFresh();
      expect(screen.queryByTestId('onb-intro')).toBeNull();
      expect(heading()).toBe('Chào mừng đến với Font Atlas');
    });

    it('"Vào thư viện" plays the outro and only then calls onDone; the step is saved first', async () => {
      const api = createMockApi({ firstRun: true });
      await api.setOnboardingStep('end');
      const onDone = vi.fn();
      const save = vi.spyOn(api, 'setOnboardingStep');
      // Biên rộng: outro 1500 ms, kiểm "chưa gọi" ngay sau thao tác (máy bận cũng chỉ mất vài trăm ms), rồi chờ có hạn dài tới khi gọi.
      render(<Onboarding api={api} onDone={onDone} outroMs={1500} />);
      await settle();
      await click('Vào thư viện'); await click('Bắt đầu');
      expect(save).toHaveBeenLastCalledWith('done');
      expect(screen.getByTestId('onb-outro')).toBeTruthy();
      expect(onDone).not.toHaveBeenCalled();
      await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1), { timeout: 8000 });
    });

    it('"Bỏ qua hướng dẫn" skips the outro', async () => {
      const { onDone } = await startFresh();
      await click('Bỏ qua hướng dẫn');
      expect(onDone).toHaveBeenCalledTimes(1);
      expect(screen.queryByTestId('onb-outro')).toBeNull();
    });

    it('the step indicator is plain dots (group label only: no "Bước n/total" text, no key hints); the stage and the text are separate regions', async () => {
      await mount();
      expect(document.querySelector('.fa-onb-stepno')).toBeNull();
      expect(document.querySelector('.fa-onb-keys')).toBeNull();
      expect(document.querySelector('.fa-onb-top')!.textContent).not.toMatch(/Bước|Esc|←|→/);
      expect(document.querySelectorAll('.fa-onb-dots i').length).toBeGreaterThanOrEqual(6);
      expect(screen.getByRole('group', { name: /^Bước 1\/\d$/ })).toBeTruthy();
      expect(document.querySelector('.fa-onb-body > .fa-stage')).toBeTruthy();
      expect(document.querySelector('.fa-onb-body > .fa-onb-pane h1')).toBeTruthy();
    });

    it('the last step shows the shortcuts overlay BEFORE the outro; Esc skips it and finishes', async () => {
      const api = createMockApi({ firstRun: true });
      await api.setOnboardingStep('end');
      const onDone = vi.fn();
      const save = vi.spyOn(api, 'setOnboardingStep');
      render(<Onboarding api={api} onDone={onDone} outroMs={50} />);
      await settle();
      expect(screen.queryByRole('dialog')).toBeNull();
      await click('Vào thư viện');
      expect(screen.getByRole('dialog', { name: 'Phím tắt & tính năng chính' })).toBeTruthy();
      expect(screen.getAllByTestId('tip-card').length).toBeGreaterThanOrEqual(8);
      expect(screen.queryByTestId('onb-outro')).toBeNull();
      expect(save).not.toHaveBeenCalledWith('done');
      await act(async () => { fireEvent.keyDown(document.body, { key: 'Escape' }); });
      await settle();
      expect(save).toHaveBeenLastCalledWith('done');
      expect(screen.queryByRole('dialog')).toBeNull();
      expect(screen.getByTestId('onb-outro')).toBeTruthy();
    });

    it('the illustration frame is square and has no border', async () => {
      await mount();
      const svg = document.querySelector('.fa-stage svg.fa-bp')!;
      expect(svg.getAttribute('viewBox')).toBe('0 0 480 480');
      expect(getComputedStyle(svg).borderTopWidth).not.toBe('1px');
    });

    it('keyboard: → advances only on safe buttons; the licence step is never acknowledged by a key', async () => {
      await mount();
      await act(async () => { fireEvent.keyDown(document.body, { key: 'ArrowRight' }); });
      await settle();
      expect(heading()).toBe('Thư mục font');
      await act(async () => { fireEvent.keyDown(document.body, { key: 'ArrowLeft' }); });
      await settle();
      expect(heading()).toBe('Chào mừng đến với Font Atlas');
      cleanupAll();
      const api2 = createMockApi({ firstRun: true });
      await api2.setOnboardingStep('o5');
      render(<Onboarding api={api2} onDone={() => {}} animated={false} />);
      await settle();
      await act(async () => { fireEvent.keyDown(document.body, { key: 'ArrowRight' }); fireEvent.keyDown(document.body, { key: 'Enter' }); });
      await settle();
      expect(heading()).toBe('Giấy phép và trách nhiệm');
      expect((await api2.getSettings()).licenseNoticeAck).toBe(false);
    });

    it('while the splash is open the guide ignores every key (Esc must not skip the guide, Enter must not advance)', async () => {
      const { onDone } = await mount();
      setSplashOpen(true);
      await act(async () => { fireEvent.keyDown(document.body, { key: 'Escape' }); fireEvent.keyDown(document.body, { key: 'Enter' }); fireEvent.keyDown(document.body, { key: 'ArrowRight' }); });
      await settle();
      expect(onDone).not.toHaveBeenCalled();
      expect(heading()).toBe('Chào mừng đến với Font Atlas');
      setSplashOpen(false);
      await act(async () => { fireEvent.keyDown(document.body, { key: 'ArrowRight' }); });
      await settle();
      expect(heading()).toBe('Thư mục font');
    });

    it('describes its keys for screen readers (aria-describedby)', async () => {
      await mount();
      const main = document.querySelector('.fa-onb-body')!;
      expect(document.getElementById(main.getAttribute('aria-describedby')!)!.textContent).toMatch(/Esc/);
    });

    it('keyboard: arrows inside the radios are left to the radios; Esc on step 1 skips the guide', async () => {
      const { onDone } = await mount();
      const radio = screen.getByRole('radio', { name: 'English' });
      radio.focus();
      await act(async () => { fireEvent.keyDown(radio, { key: 'ArrowRight' }); });
      expect(heading()).toBe('Chào mừng đến với Font Atlas');
      await act(async () => { fireEvent.keyDown(document.body, { key: 'Escape' }); });
      await settle();
      expect(onDone).toHaveBeenCalledTimes(1);
    });
  });
});

// nhỏ gọn: gỡ mọi thứ đã render giữa hai lần dựng trong cùng một test
import { cleanup } from '@testing-library/react';
const cleanupAll = () => cleanup();
void within;

describe('Outro keys', () => {
  it('Enter held down (key repeat) does not skip the outro; a single press does', async () => {
    const { Outro } = await import('../Outro');
    const onDone = vi.fn();
    render(<Outro onDone={onDone} ms={10_000} />);
    fireEvent.keyDown(document.body, { key: 'Enter', repeat: true });
    expect(onDone).not.toHaveBeenCalled();
    fireEvent.keyDown(document.body, { key: 'Enter' });
    expect(onDone).toHaveBeenCalledTimes(1);
  });
});
