/** Sắp xếp thư viện (spec library-view-options B5). `released` = năm phát hành ước lượng, `file_created` = ngày tạo file (R2). */
export const SORT_FIELDS = ['family', 'added', 'released', 'file_created', 'weight', 'match'] as const;
export type SortField = (typeof SORT_FIELDS)[number];
/** `explicit`: người dùng đã chọn (kể cả đúng mặc định Tên A→Z) → xếp THUẦN theo trường; chưa chọn mà đang tìm → Khớp. */
/** `group`: nhóm theo Family (R3); lưu cùng `ui.sort`, mặc định tắt. */
export type SortPrefs = { by: SortField; desc: boolean; explicit?: boolean; group?: boolean };
export const SORT_KEY = 'ui.sort'; // 6a: ^ui\.[a-z0-9_.-]{1,60}$
export const DEFAULT_SORT: SortPrefs = { by: 'family', desc: false };
/** Đổi trường thì chiều về mặc định của trường đó: tên và độ đậm tăng dần, các trường ngày/năm mới → cũ. */
export const defaultDesc = (by: SortField): boolean => by === 'added' || by === 'released' || by === 'file_created';
export const isDefaultSort = (s: SortPrefs): boolean => s.by === DEFAULT_SORT.by && s.desc === DEFAULT_SORT.desc;
/** Còn đúng mặc định cả trường sắp xếp lẫn nhóm theo Family. */
export const isDefaultPrefs = (s: SortPrefs): boolean => isDefaultSort(s) && !s.explicit && !s.group;
/** Sắp xếp áp dụng thật cho truy vấn: Khớp khi đang tìm mà người dùng chưa chọn; Khớp mà không có chữ tìm thì về mặc định. */
export function effectiveSort(s: SortPrefs, hasQuery: boolean): SortPrefs {
  if (s.by === 'match') return hasQuery ? s : { ...DEFAULT_SORT, ...(s.group ? { group: true } : {}) };
  if (hasQuery && !s.explicit && isDefaultSort(s)) return { by: 'match', desc: false };
  return s;
}

/** Dữ liệu hỏng / khoá lạ → mặc định. `match` không lưu (chỉ có nghĩa khi đang tìm, phiên sau không có chữ tìm). */
export function parseSort(raw: string | null | undefined): SortPrefs {
  if (!raw) return { ...DEFAULT_SORT };
  try {
    const o = JSON.parse(raw) as { v?: unknown; by?: unknown; desc?: unknown; group?: unknown };
    if (o?.v !== 1 || typeof o.by !== 'string' || !(SORT_FIELDS as readonly string[]).includes(o.by) || o.by === 'match') return { ...DEFAULT_SORT, ...(o?.v === 1 && o.group === true ? { group: true } : {}) };
    return { by: o.by as SortField, desc: typeof o.desc === 'boolean' ? o.desc : defaultDesc(o.by as SortField), ...(o.group === true ? { group: true } : {}) };
  } catch { return { ...DEFAULT_SORT }; }
}
export const serializeSort = (s: SortPrefs): string => JSON.stringify({ v: 1, by: s.by === 'match' ? DEFAULT_SORT.by : s.by, desc: s.by === 'match' ? false : s.desc, ...(s.group ? { group: true } : {}) });
