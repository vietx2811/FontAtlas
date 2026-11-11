/** Sắp xếp thư viện (spec library-view-options B5). Chỉ các trường backend đã hỗ trợ; năm phát hành / ngày file chờ R2. */
export const SORT_FIELDS = ['family', 'added', 'weight', 'match'] as const;
export type SortField = (typeof SORT_FIELDS)[number];
export type SortPrefs = { by: SortField; desc: boolean };
export const SORT_KEY = 'ui.sort'; // 6a: ^ui\.[a-z0-9_.-]{1,60}$
export const DEFAULT_SORT: SortPrefs = { by: 'family', desc: false };
/** Đổi trường thì chiều về mặc định của trường đó: tên và độ đậm tăng dần, ngày thêm mới → cũ. */
export const defaultDesc = (by: SortField): boolean => by === 'added';
export const isDefaultSort = (s: SortPrefs): boolean => s.by === DEFAULT_SORT.by && s.desc === DEFAULT_SORT.desc;

/** Dữ liệu hỏng / khoá lạ → mặc định. `match` không lưu (chỉ có nghĩa khi đang tìm, phiên sau không có chữ tìm). */
export function parseSort(raw: string | null | undefined): SortPrefs {
  if (!raw) return { ...DEFAULT_SORT };
  try {
    const o = JSON.parse(raw) as { v?: unknown; by?: unknown; desc?: unknown };
    if (o?.v !== 1 || typeof o.by !== 'string' || !(SORT_FIELDS as readonly string[]).includes(o.by) || o.by === 'match') return { ...DEFAULT_SORT };
    return { by: o.by as SortField, desc: typeof o.desc === 'boolean' ? o.desc : defaultDesc(o.by as SortField) };
  } catch { return { ...DEFAULT_SORT }; }
}
export const serializeSort = (s: SortPrefs): string => JSON.stringify({ v: 1, by: s.by === 'match' ? DEFAULT_SORT.by : s.by, desc: s.by === 'match' ? false : s.desc });
