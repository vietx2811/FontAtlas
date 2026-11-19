// TaskHub (beta.5): khay "Tác vụ" ở thanh bên. BE là nguồn sự thật (tiến độ, huỷ); FE chỉ vẽ, không tự suy tiến độ.
// Ẩn khi rảnh, không lưu lịch sử: việc xong chỉ nằm lại ~8 s dạng ✓ rồi biến mất; lỗi báo một lần bằng toast.
import { create, type StoreApi, type UseBoundStore } from 'zustand';
import type { Api } from '../api';
import type { TaskDto, TaskEvent } from '../api/taskTypes';
import { hasKey, t } from '../i18n';
import { useNoticeStore, type NoticeStore } from './notices';

export const DONE_LINGER_MS = 8000;
/** Hơn từng này dòng cùng loại thì gộp thành một dòng. */
export const GROUP_OVER = 3;

type State = {
  rev: number;
  tasks: Record<string, TaskDto>;
  /** Việc vừa xong, giữ DONE_LINGER_MS để người dùng thấy ✓ (không phải lịch sử). */
  done: Record<string, TaskDto>;
};
export type TaskHubStore = UseBoundStore<StoreApi<State>>;
export const createTaskHubStore = (): TaskHubStore => create<State>()(() => ({ rev: 0, tasks: {}, done: {} }));
export const useTaskHubStore = createTaskHubStore();

/** Watcher chưa biết có đổi gì không: ẩn tới khi có tiến độ (tránh khay nháy mỗi lần quét không đổi). */
export const isShown = (x: TaskDto): boolean => x.trigger !== 'watcher' || x.done > 0;
export const shownTasks = (s: State): TaskDto[] => Object.values(s.tasks).filter(isShown).sort((a, b) => a.startedAt - b.startedAt || a.id.localeCompare(b.id));

export type TaskRow = {
  key: string; kind: TaskDto['kind']; tasks: TaskDto[]; done: number; total: number | null;
  state: TaskDto['state'] | 'done'; cancellable: boolean; confirmKey?: string; finished: boolean;
};
const frac = (x: { done: number; total: number | null }): number | null => (x.total && x.total > 0 ? Math.min(1, x.done / x.total) : null);

/** Gộp các dòng cùng loại khi quá GROUP_OVER; dòng gộp cộng done/total (total null nếu có phần chưa biết). */
export function buildRows(running: TaskDto[], finished: TaskDto[]): TaskRow[] {
  const rows: TaskRow[] = [];
  const byKind = new Map<string, TaskDto[]>();
  for (const x of running) byKind.set(x.kind, [...(byKind.get(x.kind) ?? []), x]);
  for (const x of running) {
    const same = byKind.get(x.kind)!;
    if (same.length <= GROUP_OVER) { rows.push({ key: x.id, kind: x.kind, tasks: [x], done: x.done, total: x.total, state: x.state, cancellable: x.cancellable, confirmKey: x.cancelConfirmKey, finished: false }); continue; }
    if (rows.some(r => r.key === `group:${x.kind}`)) continue;
    const known = same.every(y => y.total !== null);
    rows.push({
      key: `group:${x.kind}`, kind: x.kind, tasks: same, done: same.reduce((a, y) => a + y.done, 0), total: known ? same.reduce((a, y) => a + (y.total ?? 0), 0) : null,
      state: same.every(y => y.state === 'cancelling') ? 'cancelling' : same.some(y => y.state === 'running') ? 'running' : 'queued',
      cancellable: same.some(y => y.cancellable && y.state !== 'cancelling'), confirmKey: same.find(y => y.cancelConfirmKey)?.cancelConfirmKey, finished: false,
    });
  }
  for (const x of finished) rows.push({ key: `done:${x.id}`, kind: x.kind, tasks: [x], done: x.total ?? x.done, total: x.total, state: 'done', cancellable: false, finished: true });
  return rows;
}

/** Tóm tắt cho nút: tiến độ trung bình các việc đã biết tổng; null = không xác định (vòng quay). */
export function summarize(running: TaskDto[]): { count: number; progress: number | null; done: number; total: number | null; single: boolean } {
  const fr = running.map(frac).filter((v): v is number => v !== null);
  const progress = fr.length ? fr.reduce((a, b) => a + b, 0) / fr.length : null;
  const single = running.length === 1;
  return { count: running.length, progress, done: single ? running[0].done : 0, total: single ? running[0].total : null, single };
}

/** Tên hiển thị của việc: khoá BE gửi (titleKey) nếu app có bản dịch, không thì tên loại việc. */
export const taskTitle = (x: Pick<TaskDto, 'titleKey' | 'titleArgs' | 'kind'>): string =>
  hasKey(x.titleKey) ? t(x.titleKey, x.titleArgs) : hasKey(`task.kind.${x.kind}`) ? t(`task.kind.${x.kind}` as never) : x.kind;

/**
 * Nối với BE: subscribe TRƯỚC rồi list (đệm sự kiện tới lúc list về), bỏ sự kiện có rev ≤ rev đã có.
 * Hub chỉ để hiển thị: `removed done` KHÔNG bump thư viện (bump theo ScanEvent.finished.changed).
 */
export async function startTaskHub(api: Api, store: TaskHubStore = useTaskHubStore, notices: NoticeStore = useNoticeStore): Promise<() => void> {
  const timers = new Set<ReturnType<typeof setTimeout>>();
  let alive = true;
  let ready = false;
  const buffer: TaskEvent[] = [];

  const apply = (e: TaskEvent) => {
    const s = store.getState();
    if (e.type === 'upsert') {
      if (e.task.rev <= s.rev) return;
      store.setState({ rev: e.task.rev, tasks: { ...s.tasks, [e.task.id]: e.task } });
      return;
    }
    if (e.rev <= s.rev) return;
    const prev = s.tasks[e.id];
    const { [e.id]: _gone, ...rest } = s.tasks;
    const next: Partial<State> = { rev: e.rev, tasks: rest };
    if (e.outcome === 'done' && prev && isShown(prev)) {
      next.done = { ...s.done, [e.id]: prev };
      const timer = setTimeout(() => { timers.delete(timer); store.setState(c => { const { [e.id]: _d, ...left } = c.done; return { done: left }; }); }, DONE_LINGER_MS);
      timers.add(timer);
    }
    store.setState(next);
    if (e.outcome === 'failed' && prev) {
      const name = taskTitle(prev);
      const detail = e.errorKey ? (hasKey(e.errorKey) ? t(e.errorKey) : undefined) : undefined;
      notices.getState().push({ kind: 'error', key: `task:${e.id}`, text: t('task.failed', { name }), detail });
    }
  };

  const off = await api.taskSubscribe(e => { if (!alive) return; if (ready) apply(e); else buffer.push(e); });
  try {
    const list = await api.taskList();
    if (alive) store.setState({ rev: list.rev, tasks: Object.fromEntries(list.tasks.map(x => [x.id, x])) });
  } catch (err) { console.warn('list_tasks failed', err); }
  ready = true;
  buffer.splice(0).forEach(e => { if (alive) apply(e); });
  return () => { alive = false; off(); timers.forEach(clearTimeout); timers.clear(); store.setState({ rev: 0, tasks: {}, done: {} }); };
}
