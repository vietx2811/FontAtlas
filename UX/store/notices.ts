// Thông báo gọn, không modal (UX 03 §7.2): tối đa 4, trùng nội dung thì không thêm.
import { create, type StoreApi, type UseBoundStore } from 'zustand';

export type Notice = { id: number; kind: 'info' | 'error'; /** Nhóm: thông báo mới cùng khoá thay thông báo cũ; `dismissKey` gỡ cả nhóm (vd font của Case tắt → toast "đã bật" của Case đó biến mất). */ key?: string; /** Chi tiết phụ: hiện thành dấu "!" có tooltip, không thêm chữ vào dòng chính. */ detail?: string; text: string; action?: { label: string; run(): void }; /** Nút thứ hai (vd "Cài đặt…" cạnh "Tắt lần này"); không tự đóng thông báo. */ action2?: { label: string; run(): void } };
type State = { notices: Notice[]; push(n: Omit<Notice, 'id'>): number; dismiss(id: number): void; dismissKey(key: string): void };
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
      set(s => ({ notices: [...s.notices.filter(x => !n.key || x.key !== n.key), { ...n, id }].slice(-MAX) }));
      return id;
    },
    dismiss: id => set(s => ({ notices: s.notices.filter(n => n.id !== id) })),
    dismissKey: key => set(s => (s.notices.some(n => n.key === key) ? { notices: s.notices.filter(n => n.key !== key) } : s)),
  }));
}
export const useNoticeStore = createNoticeStore();
