// Thông báo gọn, không modal (UX 03 §7.2): tối đa 4, trùng nội dung thì không thêm.
import { create, type StoreApi, type UseBoundStore } from 'zustand';

export type Notice = { id: number; kind: 'info' | 'error'; text: string; action?: { label: string; run(): void } };
type State = { notices: Notice[]; push(n: Omit<Notice, 'id'>): number; dismiss(id: number): void };
export type NoticeStore = UseBoundStore<StoreApi<State>>;
const MAX = 4;

export function createNoticeStore(): NoticeStore {
  let next = 1;
  return create<State>()((set, get) => ({
    notices: [],
    push(n) {
      const dup = get().notices.find(x => x.kind === n.kind && x.text === n.text);
      if (dup) return dup.id;
      const id = next++;
      set(s => ({ notices: [...s.notices, { ...n, id }].slice(-MAX) }));
      return id;
    },
    dismiss: id => set(s => ({ notices: s.notices.filter(n => n.id !== id) })),
  }));
}
export const useNoticeStore = createNoticeStore();
