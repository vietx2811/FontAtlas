// TaskHub (beta.5): kiểu khớp docs/design/taskhub-be-beta5.md. Khi BE (02) sinh bindings ts-rs thì thay file này bằng `export type … from '../bindings'`.
export type TaskKind = 'scan' | 'analyze' | 'embed' | 'cloudDownload' | 'activation' | 'resolveDuplicates' | 'export' | 'other';
export type TaskTrigger = 'watcher' | 'user' | 'launch';
export type TaskUnit = 'items' | 'percent' | 'bytes';
export type TaskState = 'queued' | 'running' | 'cancelling';
export type TaskOutcome = 'done' | 'cancelled' | 'failed';

export type TaskDto = {
  id: string;
  /** Bộ đếm đơn điệu toàn hub; tăng ở mọi upsert và removed. */
  rev: number;
  kind: TaskKind;
  trigger: TaskTrigger;
  titleKey: string;
  /** Không chứa đường dẫn đầy đủ, chỉ tên nguồn/Case. */
  titleArgs: Record<string, string | number>;
  done: number;
  /** null = chưa biết (không bao giờ dùng 0). */
  total: number | null;
  unit: TaskUnit;
  state: TaskState;
  cancellable: boolean;
  /** Có = hỏi xác nhận nhỏ trong dòng trước khi huỷ (nội dung là khoá i18n). */
  cancelConfirmKey?: string;
  /** Vd kind+nguồn; FE gộp khi hơn 3 dòng cùng kind. */
  groupKey?: string;
  /** epoch ms. */
  startedAt: number;
};

export type TaskListDto = { rev: number; tasks: TaskDto[] };
export type TaskEvent =
  | { type: 'upsert'; task: TaskDto }
  | { type: 'removed'; id: string; rev: number; outcome: TaskOutcome; errorKey?: string | null };
