import { create } from 'zustand';
import type { Api, ReasonDto } from '../api';

/** Tên Dự án/Case và app gắn Case, tra một lần rồi nhớ: dòng "vì sao đang bật" cần tên, còn activation_state chỉ có id. */
type State = { names: Record<string, string>; set(k: string, v: string): void };
export const useReasonNames = create<State>()(set => ({ names: {}, set: (k, v) => set(s => ({ names: { ...s.names, [k]: v } })) }));

const inflight = new Set<string>();
/** Gọi khi thấy một lý do chưa có tên; trả về ngay, tên về sau qua store. Lỗi: giữ nguyên (hiện số). */
export function resolveReasonNames(api: Api, r: ReasonDto): void {
  const put = (k: string, load: () => Promise<string>) => { if (useReasonNames.getState().names[k] !== undefined || inflight.has(k)) return; inflight.add(k); load().then(v => useReasonNames.getState().set(k, v), () => undefined).finally(() => inflight.delete(k)); };
  // `c:` = id của collection/Dự án, `k:` = id của CASE: hai không gian id khác nhau, không dùng chung khoá (đụng số là sai tên).
  // Đây chỉ là đường DỰ PHÒNG khi `activation_state` chưa kèm `reasonNames` (bình thường tên về cùng trạng thái, khỏi gọi lẻ).
  if (r.kind === 'collection') {
    const id = r.collectionId;
    put(`c:${id}`, async () => { const c = (await api.listCollections()).find(x => x.id === id); if (!c) throw new Error('gone'); return c.name; });
  } else if (r.kind === 'case') {
    const id = r.caseId;
    put(`k:${id}`, async () => (await api.getCase(id)).name);
  } else if (r.kind === 'case_app') {
    const id = r.caseId;
    put(`k:${id}`, async () => (await api.getCase(id)).name);
    put(`a:${id}`, async () => (await api.caseApps(id)).map(a => a.name).join(', '));
  }
}
