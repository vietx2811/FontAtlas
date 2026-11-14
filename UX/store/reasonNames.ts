import { create } from 'zustand';
import type { Api, ReasonDto } from '../api';

/** Tên Dự án/Case và app gắn Case, tra một lần rồi nhớ: dòng "vì sao đang bật" cần tên, còn activation_state chỉ có id. */
type State = { names: Record<string, string>; set(k: string, v: string): void };
export const useReasonNames = create<State>()(set => ({ names: {}, set: (k, v) => set(s => ({ names: { ...s.names, [k]: v } })) }));

const inflight = new Set<string>();
/** Gọi khi thấy một lý do chưa có tên; trả về ngay, tên về sau qua store. Lỗi: giữ nguyên (hiện số). */
export function resolveReasonNames(api: Api, r: ReasonDto): void {
  const put = (k: string, load: () => Promise<string>) => { if (useReasonNames.getState().names[k] !== undefined || inflight.has(k)) return; inflight.add(k); load().then(v => useReasonNames.getState().set(k, v), () => undefined).finally(() => inflight.delete(k)); };
  if (r.kind === 'collection' || r.kind === 'case_app') {
    const id = r.collectionId;
    put(`c:${id}`, async () => { try { return (await api.getCase(id)).name; } catch { const c = (await api.listCollections()).find(x => x.id === id); if (!c) throw new Error('gone'); return c.name; } });
    if (r.kind === 'case_app') put(`a:${id}`, async () => (await api.caseApps(id)).map(a => a.name).join(', '));
  }
}
