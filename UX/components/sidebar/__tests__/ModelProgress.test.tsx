import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../../api/mock';
import { createModelStore } from '../../../store/model';
import { ModelProgress, SR_THROTTLE_MS } from '../ModelProgress';

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());
const run = (ms: number) => act(async () => { await vi.advanceTimersByTimeAsync(ms); });

async function mount(init: Parameters<typeof createMockApi>[0] = {}, embedEnabled = true) {
  const api = createMockApi({ tickMs: 100, ...init });
  if (embedEnabled) await api.setSettings({ embedEnabled: true });
  const store = createModelStore();
  await act(async () => { await store.getState().sync(api); });
  render(<ModelProgress api={api} store={store} />);
  return { api, store };
}

describe('ModelProgress', () => {
  it('E0: renders nothing', async () => {
    await mount({}, false);
    expect(screen.queryByTestId('model-progress')).toBeNull();
  });

  it('E1: one line with [Tải], closable; [Tải] starts the download (E2 with Dừng)', async () => {
    const { store } = await mount();
    expect(screen.getByTestId('model-progress').textContent).toContain('Chưa tải model hình ảnh');
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Tải' })); });
    await run(250);
    expect(screen.getByTestId('model-progress').textContent).toMatch(/Đang tải model \d+% · \d+\/371 MB/);
    expect(screen.getByRole('button', { name: 'Dừng' })).toBeTruthy();
    expect(store.getState().ui().code).toBe('E2');
  });

  it('E1 can be dismissed', async () => {
    await mount();
    fireEvent.click(screen.getByRole('button', { name: 'Đóng gợi ý' }));
    expect(screen.queryByTestId('model-progress')).toBeNull();
  });

  it('E2 → Dừng → E3 → Tiếp tục tải', async () => {
    await mount();
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Tải' })); });
    await run(300);
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Dừng' })); });
    expect(screen.getByTestId('model-progress').textContent).toContain('Đã dừng tải model');
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Tiếp tục tải' })); });
    expect(screen.getByTestId('model-progress').textContent).toContain('Đang tải model');
  });

  it('E8 shows the progress ring and device in the tooltip; ⏸ pauses to E9', async () => {
    await mount({ initial: { model: { state: 'ready' }, embed: { enabled: true, running: true, pending: 3000, done: 800, device: 'metal', etaMs: 120_000 } } });
    const row = screen.getByTestId('model-progress');
    expect(row.textContent).toContain('Phân tích hình ảnh 800/3.800');
    expect(row.getAttribute('title')).toBe('Đang dùng: Metal (GPU)');
    expect(row.querySelector('[data-ring]')).toBeTruthy();
    await act(async () => { fireEvent.click(screen.getByRole('button', { name: 'Tạm dừng' })); });
    expect(screen.getByTestId('model-progress').textContent).toContain('tạm dừng');
    expect(screen.getByRole('button', { name: 'Tiếp tục' })).toBeTruthy();
  });

  it('errors are announced assertively and carry a retry action', async () => {
    await mount({ initial: { model: { state: 'checksum_mismatch' } } });
    const alert = screen.getByRole('alert');
    expect(alert.textContent).toContain('Model tải về bị hỏng');
    expect(screen.getByRole('button', { name: 'Tải lại' })).toBeTruthy();
  });

  it('E6 offers both [Tải lại] and [Xoá model]', async () => {
    await mount({ initial: { model: { state: 'corrupt' } } });
    expect(screen.getByRole('button', { name: 'Tải lại' })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Xoá model' })).toBeTruthy();
  });

  it('progress is announced politely at most once every 10 s', async () => {
    await mount({ tickMs: 2000, initial: { model: { state: 'ready' }, embed: { enabled: true, running: true, pending: 3600, done: 200, device: 'metal', etaMs: 120_000 } } });
    const live = () => screen.getByTestId('model-live').textContent;
    expect(screen.getByTestId('model-live').getAttribute('aria-live')).toBe('polite');
    const first = live();
    expect(first).toContain('hình ảnh 200/3.800');
    await run(SR_THROTTLE_MS - 1000); // ~9 s: giao diện đã nhảy số nhưng vùng đọc màn hình chưa
    expect(screen.getByTestId('model-progress').querySelector('.fa-model-text')!.textContent).not.toContain('hình ảnh 200/3.800');
    expect(live()).toBe(first);
    await run(2000);
    expect(live()).not.toBe(first);
  });

  it('the rail (collapsed sidebar) keeps the ring and buttons by accessible name', async () => {
    await mount({ initial: { model: { state: 'ready' }, embed: { enabled: true, running: true, pending: 3000, done: 800, device: 'cpu' } } });
    expect(screen.getByRole('button', { name: 'Tạm dừng' })).toBeTruthy();
    expect(screen.getByTestId('model-progress').getAttribute('data-code')).toBe('E8');
  });
});
