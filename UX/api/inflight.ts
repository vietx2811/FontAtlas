import type { Api } from './index';

/** Lệnh chỉ đọc có thể dùng chung một lần gọi đang bay khi tham số y hệt. */
const READS = new Set(['listSources', 'listCollections', 'listTags', 'listCases', 'duplicateSummary', 'queryFaces', 'familyGroups']);

/**
 * Gộp các lệnh ĐỌC giống hệt nhau đang chạy cùng lúc thành một IPC (nhiều component cùng nạp khi khởi động/bump quét, StrictMode chạy đôi effect).
 * An toàn: chỉ gộp khi lần gọi trước CHƯA xong; mọi lệnh khác (có thể là ghi) gọi vào thì bỏ hết bản đang bay, để đọc sau ghi luôn là lần đọc mới.
 */
export function withInflightDedupe<T extends Api>(api: T): T {
  const pending = new Map<string, Promise<unknown>>();
  return new Proxy(api, {
    get(t, k, r) {
      const v = Reflect.get(t, k, r);
      if (typeof v !== 'function') return v;
      const name = String(k);
      if (!READS.has(name)) return (...a: unknown[]) => { pending.clear(); return (v as (...x: unknown[]) => unknown).apply(t, a); };
      return (...a: unknown[]) => {
        let key: string;
        try { key = `${name}:${JSON.stringify(a)}`; } catch { return (v as (...x: unknown[]) => unknown).apply(t, a); }
        const hit = pending.get(key);
        if (hit) return hit;
        const p = (v as (...x: unknown[]) => Promise<unknown>).apply(t, a);
        pending.set(key, p);
        const done = () => { if (pending.get(key) === p) pending.delete(key); };
        p.then(done, done);
        return p;
      };
    },
  }) as T;
}
