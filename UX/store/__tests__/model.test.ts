import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../api/mock';
import type { EmbedStatusDto, ModelStatusDto } from '../../api';
import { createModelStore, deriveModelUi, formatEta, fmtInt } from '../model';

const MB = 1024 * 1024;
const model = (o: Partial<ModelStatusDto> = {}): ModelStatusDto => ({
  state: 'ready', doneBytes: 371 * MB, totalBytes: 371 * MB, bytesPerSec: 0, etaMs: null, location: '/m', diskFree: 1e11, requiredBytes: 408 * MB, lastError: null, ...o,
});
const embed = (o: Partial<EmbedStatusDto> = {}): EmbedStatusDto => ({
  enabled: true, running: false, paused: false, pending: 0, done: 0, failed: 0, device: null, msPerFace: null, etaMs: null, modelVersion: 'siglip2', loadError: null, ...o,
});
const ui = (m: ModelStatusDto | null, e: EmbedStatusDto | null, extra: { embedEnabled?: boolean; justFinished?: boolean; e1Dismissed?: boolean } = {}) =>
  deriveModelUi({ embedEnabled: true, justFinished: false, e1Dismissed: false, model: m, embed: e, ...extra });

describe('formatting', () => {
  it('thousands use a dot, ETA reads in minutes', () => {
    expect(fmtInt(3800)).toBe('3.800');
    expect(fmtInt(412)).toBe('412');
    expect(formatEta(30_000)).toBe('dưới 1 phút');
    expect(formatEta(60_000)).toBe('~1 phút');
    expect(formatEta(130_000)).toBe('~2 phút');
    expect(formatEta(3 * 3600_000)).toBe('~3 giờ');
    expect(formatEta(null)).toBe('');
  });
});

describe('deriveModelUi — one case per UX 03 §3.1 state', () => {
  it('E0 disabled: nothing in the footer', () => {
    const u = ui(model({ state: 'missing' }), embed({ enabled: false }), { embedEnabled: false });
    expect([u.code, u.text]).toEqual(['E0', null]);
    expect(u.imageReady).toBe(false);
  });
  it('E1 enabled but model missing: one line with [Tải], dismissible for the session', () => {
    const u = ui(model({ state: 'missing', doneBytes: 0 }), embed());
    expect([u.code, u.text, u.actions.map(a => a.id)]).toEqual(['E1', 'Chưa tải model hình ảnh', ['download']]);
    expect(ui(model({ state: 'missing' }), embed(), { e1Dismissed: true }).text).toBeNull();
  });
  it('E2 downloading: percent, MB, ETA and [Dừng]', () => {
    const u = ui(model({ state: 'downloading', doneBytes: 140 * MB, etaMs: 60_000 }), embed());
    expect(u.code).toBe('E2');
    expect(u.text).toBe('Đang tải model 38% · 140/371 MB · ~1 phút');
    expect(u.actions.map(a => a.id)).toEqual(['stop']);
    expect(u.progress).toBeCloseTo(140 / 371);
  });
  it('E3 stopped (partial file kept): [Tiếp tục tải]', () => {
    const u = ui(model({ state: 'paused', doneBytes: 100 * MB }), embed());
    expect([u.code, u.text, u.actions.map(a => a.id)]).toEqual(['E3', 'Đã dừng tải model', ['download']]);
    expect(u.actions[0].label).toBe('Tiếp tục tải');
  });
  it('E4 network error (also when the last error is io/network on a missing or paused model): [Thử lại]', () => {
    for (const state of ['missing', 'paused'] as const) {
      const u = ui(model({ state, lastError: { code: 'network', message: 'x' } }), embed());
      expect([u.code, u.text, u.severity, u.actions.map(a => a.id)]).toEqual(['E4', 'Tải model lỗi mạng', 'error', ['download']]);
    }
    expect(ui(model({ state: 'missing', lastError: { code: 'io', message: 'x' } }), embed()).code).toBe('E4');
  });
  it('E5 checksum', () => {
    const u = ui(model({ state: 'checksum_mismatch' }), embed());
    expect([u.code, u.text, u.severity, u.actions.map(a => a.id)]).toEqual(['E5', 'Model tải về bị hỏng', 'error', ['redownload']]);
  });
  it('E6 corrupt on disk: [Tải lại] [Xoá model]', () => {
    const u = ui(model({ state: 'corrupt' }), embed());
    expect([u.code, u.text, u.actions.map(a => a.id)]).toEqual(['E6', 'Model hình ảnh bị hỏng', ['redownload', 'remove']]);
  });
  it('E7 cannot load the model: error code and [Thử lại]', () => {
    const u = ui(model(), embed({ loadError: 'load_failed' }));
    expect([u.code, u.text, u.severity, u.actions.map(a => a.id)]).toEqual(['E7', 'Không chạy được phân tích hình ảnh', 'error', ['retry']]);
    expect(u.detail).toBe('load_failed');
  });
  it('E8 analysing: done/total, ETA, device, [Tạm dừng]', () => {
    const u = ui(model(), embed({ running: true, done: 412, pending: 3388, etaMs: 120_000, device: 'metal' }));
    expect(u.code).toBe('E8');
    expect(u.text).toBe('Phân tích hình ảnh 412/3.800 · ~2 phút');
    expect(u.actions.map(a => a.id)).toEqual(['pause']);
    expect(u.detail).toBe('Đang dùng: Metal (GPU)');
    expect(ui(model(), embed({ running: true, pending: 10, done: 1, device: 'cpu' })).detail).toBe('Đang dùng: CPU');
  });
  it('E9 paused analysis: [Tiếp tục]', () => {
    const u = ui(model(), embed({ running: true, paused: true, done: 412, pending: 3388 }));
    expect([u.code, u.text, u.actions.map(a => a.id)]).toEqual(['E9', 'Phân tích hình ảnh tạm dừng 412/3.800', ['resume']]);
  });
  it('E10 finished: a transient line, then nothing', () => {
    const u = ui(model(), embed({ done: 3800 }), { justFinished: true });
    expect([u.code, u.text]).toEqual(['E10', 'Đã phân tích hình ảnh 3.800 font']);
    expect(ui(model(), embed({ done: 3800 })).text).toBeNull();
  });
  it('E11 some fonts failed: nothing in the footer, a Settings line instead', () => {
    const u = ui(model(), embed({ done: 3788, failed: 12 }));
    expect([u.code, u.text]).toEqual(['E11', null]);
    expect(u.settingsText).toBe('12 font không phân tích được hình ảnh');
  });
  it('imageReady is true only when the model is ready and enabled', () => {
    expect(ui(model(), embed({ done: 5 })).imageReady).toBe(true);
    expect(ui(model({ state: 'downloading' }), embed()).imageReady).toBe(false);
  });
});

describe('model store (mock Channel)', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());
  const run = (ms: number) => vi.advanceTimersByTimeAsync(ms);

  it('enable → download → E2 … E8 … E10 → E0, driven by the simulated channels', async () => {
    const api = createMockApi({ tickMs: 100 });
    const s = createModelStore();
    const stop = await s.getState().sync(api);
    expect(s.getState().ui().code).toBe('E0');
    await s.getState().enable(api);
    expect(s.getState().ui().code).toBe('E2');
    await run(100 * 5);
    expect(s.getState().ui().code).toBe('E2');
    await run(100 * 6);
    expect(s.getState().ui().code).toBe('E8');
    await run(100 * 30);
    expect(s.getState().ui().code).toBe('E10');
    await run(5100);
    expect(s.getState().ui().code).toBe('E0');
    stop();
  });

  it('stop leaves E3 with the partial file; resume continues', async () => {
    const api = createMockApi({ tickMs: 100, initial: {} });
    const s = createModelStore();
    await s.getState().sync(api);
    await s.getState().enable(api);
    await run(300);
    await s.getState().act(api, 'stop');
    expect(s.getState().ui().code).toBe('E3');
    await s.getState().act(api, 'download');
    expect(s.getState().ui().code).toBe('E2');
  });

  it('pause / resume the analysis through the actions', async () => {
    const api = createMockApi({ tickMs: 100, initial: { model: { state: 'ready' }, embed: { enabled: true, running: true, pending: 3000, done: 800, device: 'metal' } } });
    await api.setSettings({ embedEnabled: true });
    const s = createModelStore();
    await s.getState().sync(api);
    expect(s.getState().ui().code).toBe('E8');
    await s.getState().act(api, 'pause');
    expect(s.getState().ui().code).toBe('E9');
    await s.getState().act(api, 'resume');
    expect(s.getState().ui().code).toBe('E8');
  });

  it('E1 can be dismissed for the session', async () => {
    const api = createMockApi();
    await api.setSettings({ embedEnabled: true });
    const s = createModelStore();
    await s.getState().sync(api);
    expect(s.getState().ui().code).toBe('E1');
    s.getState().dismissE1();
    expect(s.getState().ui().text).toBeNull();
  });
});

import { useNoticeStore } from '../notices';
describe('model store errors become notices', () => {
  beforeEach(() => { vi.useFakeTimers(); useNoticeStore.setState({ notices: [] }); });
  afterEach(() => vi.useRealTimers());

  it('redownload while a download runs says to stop first (busy)', async () => {
    const api = createMockApi({ tickMs: 100 });
    const s = createModelStore();
    await s.getState().sync(api);
    await s.getState().enable(api);
    await s.getState().act(api, 'redownload');
    const n = useNoticeStore.getState().notices;
    expect(n).toHaveLength(1);
    expect(n[0].text).toMatch(/đang tải/i);
    expect(n[0].text).toMatch(/dừng/);
    expect(s.getState().ui().code).toBe('E2');
  });

  it('insufficient_disk reports required and free space', async () => {
    const { CmdError } = await import('../../api');
    const api = createMockApi();
    api.modelDownload = async () => { throw new CmdError('insufficient_disk', 'x', { required: 408 * MB, free: 100 * MB }); };
    const s = createModelStore();
    await s.getState().sync(api);
    await s.getState().act(api, 'download');
    expect(useNoticeStore.getState().notices[0].text).toMatch(/cần 408 MB, còn 100 MB/);
  });
});
