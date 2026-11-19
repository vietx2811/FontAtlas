// TaskHub: kiểu sinh từ BE (ts-rs, UX/bindings). `TaskTrigger` là `ScanTrigger` của BE.
export type { TaskDto, TaskEvent, TaskKind, TaskListDto, TaskOutcome, TaskState, TaskUnit } from '../bindings';
export type { ScanTrigger as TaskTrigger } from '../bindings';
