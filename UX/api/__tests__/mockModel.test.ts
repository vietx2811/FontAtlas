import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { EmbedEvent, ModelEvent } from '../types';
import { createMockApi } from '../mock';

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());
const run = async (ms: number) => { await vi.advanceTimersByTimeAsync(ms); };

describe('mock model + embed simulation (Channel stand-in)', () => {
  it('starts as missing, with disk numbers and nothing queued', async () => {
    const api = createMockApi();
    const m = await api.modelStatus();
    expect(m.state).toBe('missing');
    expect(m.requiredBytes).toBeGreaterThan(m.totalBytes);
    expect((await api.embedStatus()).running).toBe(false);
  });

  it('download streams progress, then Ready; embedding starts by itself when enabled and finishes', async () => {
    const api = createMockApi({ tickMs: 100 });
    await api.setSettings({ embedEnabled: true });
    const model: ModelEvent[] = []; const embed: EmbedEvent[] = [];
    await api.embedSubscribe(e => embed.push(e));
    await api.modelDownload(e => model.push(e));
    expect((await api.modelStatus()).state).toBe('downloading');
    await run(100 * 12);
    expect(model.filter(e => e.type === 'progress').length).toBeGreaterThan(3);
    const last = model.filter((e): e is Extract<ModelEvent, { type: 'progress' }> => e.type === 'progress').at(-1)!;
    expect(last.done).toBeLessThanOrEqual(last.total);
    expect(model.at(-1)).toEqual({ type: 'ready' });
    expect((await api.modelStatus()).state).toBe('ready');
    expect((await api.embedStatus()).running).toBe(true);
    await run(100 * 60);
    expect(embed.some(e => e.kind === 'finished')).toBe(true);
    const st = await api.embedStatus();
    expect([st.running, st.pending]).toEqual([false, 0]);
    expect(st.done).toBeGreaterThan(0);
  });

  it('does not embed when image analysis is off', async () => {
    const api = createMockApi({ tickMs: 100 });
    await api.modelDownload(() => {});
    await run(100 * 12);
    expect((await api.modelStatus()).state).toBe('ready');
    expect((await api.embedStatus()).running).toBe(false);
  });

  it('cancel keeps the partial file (paused) and the next download resumes from there', async () => {
    const api = createMockApi({ tickMs: 100 });
    const ev: ModelEvent[] = [];
    await api.modelDownload(e => ev.push(e));
    await run(300);
    await api.modelCancel();
    const m = await api.modelStatus();
    expect(m.state).toBe('paused');
    expect(m.doneBytes).toBeGreaterThan(0);
    expect(ev.at(-1)).toEqual({ type: 'paused' });
    await api.modelDownload(e => ev.push(e));
    await run(100);
    const p = ev.filter((e): e is Extract<ModelEvent, { type: 'progress' }> => e.type === 'progress').at(-1)!;
    expect(p.done).toBeGreaterThan(m.doneBytes);
  });

  it('pause and resume the embed queue; remove drops the model and stops the queue', async () => {
    const api = createMockApi({ tickMs: 100, initial: { model: { state: 'ready' }, embed: { enabled: true, running: true, pending: 1000, done: 0 } } });
    await api.embedPause();
    expect((await api.embedStatus()).paused).toBe(true);
    const before = (await api.embedStatus()).done;
    await run(500);
    expect((await api.embedStatus()).done).toBe(before);
    await api.embedResume();
    await run(300);
    expect((await api.embedStatus()).done).toBeGreaterThan(before);
    await api.modelRemove();
    expect((await api.modelStatus()).state).toBe('missing');
    expect((await api.embedStatus()).running).toBe(false);
  });

  it('initial overrides let dev/QA open the app in any state', async () => {
    const api = createMockApi({ initial: { model: { state: 'checksum_mismatch', lastError: { code: 'checksum_mismatch', message: 'x' } } } });
    expect((await api.modelStatus()).state).toBe('checksum_mismatch');
  });
});
