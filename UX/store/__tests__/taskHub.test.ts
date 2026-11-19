import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createMockApi } from '../../api/mock';
import type { TaskDto, TaskEvent } from '../../api/taskTypes';
import { buildRows, createTaskHubStore, DONE_LINGER_MS, shownTasks, startTaskHub, summarize } from '../taskHub';
import { createNoticeStore } from '../notices';

const task = (id: string, o: Partial<TaskDto> = {}): TaskDto => ({
  id, rev: 1, kind: 'scan', trigger: 'user', titleKey: 'task.kind.scan', titleArgs: {}, done: 0, total: 10, unit: 'items', state: 'running', cancellable: true, startedAt: 1, ...o,
});
async function setup(list: { rev: number; tasks: TaskDto[] } = { rev: 0, tasks: [] }) {
  const api = createMockApi();
  let push: (e: TaskEvent) => void = () => {};
  vi.spyOn(api, 'taskSubscribe').mockImplementation(async cb => { push = cb; return () => {}; });
  vi.spyOn(api, 'taskList').mockResolvedValue(list);
  const store = createTaskHubStore(); const notices = createNoticeStore();
  const stop = await startTaskHub(api, store, notices);
  return { api, store, notices, push: (e: TaskEvent) => push(e), stop };
}

describe('taskHub store', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('nạp danh sách đầu, rồi upsert theo rev; sự kiện rev cũ bị bỏ', async () => {
    const { store, push } = await setup({ rev: 5, tasks: [task('a', { rev: 5 })] });
    expect(Object.keys(store.getState().tasks)).toEqual(['a']);
    push({ type: 'upsert', task: task('a', { rev: 4, done: 9 }) }); // cũ hơn: bỏ
    expect(store.getState().tasks.a.done).toBe(0);
    push({ type: 'upsert', task: task('a', { rev: 6, done: 3 }) });
    expect(store.getState().tasks.a.done).toBe(3);
    expect(store.getState().rev).toBe(6);
  });

  it('sự kiện đến trước khi list về được đệm rồi áp theo rev (không đua)', async () => {
    const api = createMockApi();
    let push: (e: TaskEvent) => void = () => {};
    vi.spyOn(api, 'taskSubscribe').mockImplementation(async cb => { push = cb; return () => {}; });
    let release: (v: { rev: number; tasks: TaskDto[] }) => void = () => {};
    vi.spyOn(api, 'taskList').mockImplementation(() => new Promise(r => { release = r; }));
    const store = createTaskHubStore();
    const p = startTaskHub(api, store, createNoticeStore());
    await Promise.resolve(); await Promise.resolve();
    push({ type: 'upsert', task: task('a', { rev: 3, done: 1 }) }); // cũ hơn list
    push({ type: 'upsert', task: task('b', { rev: 8 }) });
    release({ rev: 5, tasks: [task('a', { rev: 5, done: 2 })] });
    await p;
    expect(store.getState().tasks.a.done).toBe(2);
    expect(store.getState().tasks.b).toBeDefined();
  });

  it('removed done: nằm lại 8 s dạng ✓ rồi biến mất; không toast; không lịch sử', async () => {
    const { store, notices, push } = await setup();
    push({ type: 'upsert', task: task('a', { rev: 1, done: 4 }) });
    push({ type: 'removed', id: 'a', rev: 2, outcome: 'done' });
    expect(store.getState().tasks.a).toBeUndefined();
    expect(store.getState().done.a).toBeDefined();
    expect(notices.getState().notices).toEqual([]);
    vi.advanceTimersByTime(DONE_LINGER_MS + 10);
    expect(store.getState().done).toEqual({});
  });

  it('removed failed: một toast ngắn + chi tiết; cancelled: im lặng', async () => {
    const { store, notices, push } = await setup();
    push({ type: 'upsert', task: task('a', { rev: 1 }) });
    push({ type: 'upsert', task: task('b', { rev: 2 }) });
    push({ type: 'removed', id: 'a', rev: 3, outcome: 'failed', errorKey: 'err.internal' });
    push({ type: 'removed', id: 'b', rev: 4, outcome: 'cancelled' });
    expect(notices.getState().notices).toHaveLength(1);
    expect(notices.getState().notices[0].kind).toBe('error');
    expect(store.getState().done).toEqual({});
  });

  it('task watcher ẩn tới khi có tiến độ', async () => {
    const { store } = await setup({ rev: 2, tasks: [task('w', { rev: 1, trigger: 'watcher', done: 0 }), task('u', { rev: 2 })] });
    expect(shownTasks(store.getState()).map(x => x.id)).toEqual(['u']);
    store.setState(s => ({ tasks: { ...s.tasks, w: { ...s.tasks.w, done: 2 } } }));
    expect(shownTasks(store.getState()).map(x => x.id).sort()).toEqual(['u', 'w']);
  });

  it('Hub không bump thư viện (removed done không đụng scan store)', async () => {
    const { push } = await setup();
    const { useScanStore } = await import('../scan');
    const v = useScanStore.getState().version;
    push({ type: 'upsert', task: task('a', { rev: 1, done: 1 }) });
    push({ type: 'removed', id: 'a', rev: 2, outcome: 'done' });
    expect(useScanStore.getState().version).toBe(v);
  });
});

describe('khoá thật của BE (mốc 1: quét)', () => {
  it('task.scan có tên theo nguồn; task.error.scan hiện làm chi tiết của toast lỗi', async () => {
    const { taskTitle } = await import('../taskHub');
    expect(taskTitle({ kind: 'scan', titleKey: 'task.scan', titleArgs: { source: 'Dự án khách' } })).toBe('Quét «Dự án khách»');
    vi.useFakeTimers();
    const { notices, push } = await setup();
    push({ type: 'upsert', task: task('a', { rev: 1, titleKey: 'task.scan', titleArgs: { source: 'X' } }) });
    push({ type: 'removed', id: 'a', rev: 2, outcome: 'failed', errorKey: 'task.error.scan' });
    expect(notices.getState().notices[0].detail).toMatch(/gián đoạn/);
    vi.useRealTimers();
  });
});

describe('buildRows / summarize', () => {
  it('quá 3 dòng cùng loại thì gộp; total null nếu có phần chưa biết; khác loại giữ riêng', () => {
    const many = ['1', '2', '3', '4'].map((i, n) => task(i, { kind: 'cloudDownload', done: n, total: 10 }));
    const rows = buildRows([...many, task('s')], []);
    expect(rows.map(r => r.key)).toEqual(['group:cloudDownload', 's']);
    expect(rows[0]).toMatchObject({ done: 6, total: 40 });
    const unknown = buildRows([...many.slice(0, 3), task('x', { kind: 'cloudDownload', total: null })], []);
    expect(unknown[0].total).toBeNull();
  });
  it('tóm tắt: trung bình phần đã biết tổng; không biết → null', () => {
    expect(summarize([task('a', { done: 5, total: 10 }), task('b', { done: 0, total: 10 })]).progress).toBeCloseTo(0.25);
    expect(summarize([task('a', { total: null })]).progress).toBeNull();
    expect(summarize([task('a', { done: 3, total: 12 })])).toMatchObject({ single: true, done: 3, total: 12 });
  });
});
