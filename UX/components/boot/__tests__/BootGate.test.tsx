import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { CmdError, type BootEvent } from '../../../api';
import { useNoticeStore } from '../../../store/notices';
import { BootGate } from '../BootGate';

const settle = async () => { for (let i = 0; i < 6; i++) await act(async () => { await new Promise(r => setTimeout(r, 0)); }); };
const app = <div data-testid="app">thư viện</div>;
beforeEach(() => useNoticeStore.setState({ notices: [] }));

describe('BootGate', { timeout: 15000 }, () => {
  it('ready and settled: straight to the library', async () => {
    render(<BootGate api={createMockApi()}>{app}</BootGate>);
    expect(await screen.findByTestId('app', {}, { timeout: 8000 })).toBeTruthy(); // chờ theo kết quả, không theo số lượt
  });

  it('first run: onboarding first, the library after "Vào thư viện"', async () => {
    const api = createMockApi({ firstRun: true });
    await api.setOnboardingStep('end');
    await api.setSettings({ uiLang: 'vi' }); // jsdom báo en-US; BootGate đọc ngôn ngữ đã lưu
    render(<BootGate api={api}>{app}</BootGate>);
    const h1 = await screen.findByRole('heading', { level: 1 }, { timeout: 8000 });
    expect(screen.queryByTestId('app')).toBeNull();
    expect(h1.textContent).toBe('Xong');
    // Onboarding có thể render lại (reload trạng thái) khi máy bận, làm nút đã tìm bị tách khỏi DOM.
    // Luôn bấm nút hiện tại cho tới khi vào thư viện; finish() là idempotent.
    await waitFor(() => {
      if (!screen.queryByTestId('app')) fireEvent.click(screen.queryByRole('button', { name: 'Bắt đầu' }) ?? screen.getByRole('button', { name: 'Vào thư viện' }));
      expect(screen.getByTestId('app')).toBeTruthy();
    }, { timeout: 8000 });
  });

  it('shows what the backend is doing until it is ready, then moves on', async () => {
    const api = createMockApi();
    let push: (e: BootEvent) => void = () => {};
    const legacy = { found: false, preview: null, imported: null, resolved: null, error: null };
    vi.spyOn(api, 'startupStatus')
      .mockResolvedValueOnce({ phase: 'starting', storeError: null, libraryRecreated: false, userBackup: null, legacy })
      .mockResolvedValue({ phase: 'ready', storeError: null, libraryRecreated: false, userBackup: null, legacy });
    vi.spyOn(api, 'bootSubscribe').mockImplementation(async l => { push = l; return () => {}; });
    render(<BootGate api={api}>{app}</BootGate>);
    await waitFor(() => expect(screen.getByRole('status').textContent).toMatch(/Đang mở thư viện/), { timeout: 8000 });
    expect(screen.queryByTestId('app')).toBeNull();
    act(() => push({ type: 'phase', phase: 'indexing' }));
    expect(screen.getByRole('status').textContent).toMatch(/chỉ mục/);
    act(() => push({ type: 'phase', phase: 'ready' }));
    expect(await screen.findByTestId('app', {}, { timeout: 8000 })).toBeTruthy();
  });

  it('a failed start shows the code and message with a retry, never a blank window', async () => {
    const api = createMockApi();
    vi.spyOn(api, 'startupStatus').mockResolvedValue({ phase: 'failed', storeError: { code: 'store_unavailable', message: 'không mở được user.db', detail: null }, libraryRecreated: false, userBackup: '/b/user.db.bak', legacy: { found: false, preview: null, imported: null, resolved: null, error: null } });
    render(<BootGate api={api}>{app}</BootGate>);
    // Chọn đúng alert lỗi khởi động; timer mock của test trước có thể đẩy thêm alert quét nguồn.
    const alert = (await screen.findByText(/store_unavailable/, {}, { timeout: 8000 })).closest('[role="alert"]') as HTMLElement;
    expect(alert).not.toBeNull();
    expect(alert.textContent).toMatch(/store_unavailable/);
    expect(alert.textContent).toMatch(/không mở được user.db/);
    expect(alert.textContent).toMatch(/\/b\/user\.db\.bak/);
    expect(screen.getByRole('button', { name: 'Thử lại' })).toBeTruthy();
    expect(screen.queryByTestId('app')).toBeNull();
  });

  it('a rebuilt library is explained once, with the reassurance that personal data is intact', async () => {
    const api = createMockApi();
    vi.spyOn(api, 'startupStatus').mockResolvedValue({ phase: 'ready', storeError: null, libraryRecreated: true, userBackup: null, legacy: { found: false, preview: null, imported: null, resolved: null, error: null } });
    render(<BootGate api={api}>{app}</BootGate>);
    await waitFor(() => expect(useNoticeStore.getState().notices.length).toBeGreaterThan(0), { timeout: 8000 });
    expect(useNoticeStore.getState().notices[0].text).toMatch(/Thư viện đã được tạo lại.*yêu thích.*còn nguyên/);
  });

  it('an error from startup_status itself is shown as a failure too', async () => {
    const api = createMockApi();
    vi.spyOn(api, 'startupStatus').mockRejectedValue(new CmdError('internal', 'boom'));
    render(<BootGate api={api}>{app}</BootGate>);
    expect((await screen.findByText(/boom/, {}, { timeout: 8000 })).closest('[role="alert"]')).not.toBeNull();
  });
});
