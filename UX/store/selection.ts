// Đa chọn trong lưới/danh sách thư viện. Giữ cả FaceRow (menu cần tên, trạng thái yêu thích, đường dẫn), khoá theo `${hashHex}:${faceIndex}`.
import { create, type StoreApi, type UseBoundStore } from 'zustand';
import type { FaceRow } from '../features/library-grid/types';
import { useUiStore } from './ui';

/** ⌘A và Shift-dải không chọn quá số này (khớp trần faceKeys của truy vấn). */
export const SELECT_MAX = 2000;

export const rowKey = (r: Pick<FaceRow, 'hashHex' | 'faceIndex'>): string => `${r.hashHex}:${r.faceIndex}`;

export interface SelectionState {
  rows: ReadonlyMap<string, FaceRow>;
  /** Index (trong truy vấn hiện tại) của thẻ mốc cho Shift-click. */
  anchor: number | null;
  /** Tăng mỗi lần clear: việc lấy dải/⌘A đang bay thấy epoch đổi thì bỏ kết quả. */
  epoch: number;
}
export interface SelectionActions {
  /** Chọn đúng các thẻ này. */
  replace(rows: readonly FaceRow[], anchor: number | null): void;
  /** Thêm vào vùng chọn hiện có. */
  add(rows: readonly FaceRow[], anchor?: number | null): void;
  /** Bật/tắt một thẻ; trả true nếu thẻ vừa được thêm. */
  toggle(row: FaceRow, index: number): boolean;
  clear(): void;
}
export type SelectionStore = UseBoundStore<StoreApi<SelectionState & SelectionActions>>;

const toMap = (rows: readonly FaceRow[]): ReadonlyMap<string, FaceRow> => new Map(rows.map(r => [rowKey(r), r] as const));

export function createSelectionStore(): SelectionStore {
  return create<SelectionState & SelectionActions>()((set, get) => ({
    rows: new Map(), anchor: null, epoch: 0,
    replace: (rows, anchor) => set({ rows: toMap(rows), anchor }),
    add: (rows, anchor) => set(s => ({ rows: new Map([...s.rows, ...toMap(rows)]), anchor: anchor === undefined ? s.anchor : anchor })),
    toggle(row, index) {
      const next = new Map(get().rows);
      const k = rowKey(row);
      const adding = !next.has(k);
      if (adding) next.set(k, row); else next.delete(k);
      set({ rows: next, anchor: index });
      return adding;
    },
    clear: () => set(s => ({ rows: s.rows.size ? new Map() : s.rows, anchor: null, epoch: s.epoch + 1 })),
  }));
}
export const useSelectionStore = createSelectionStore();

/** Bỏ chọn: xoá vùng chọn và đóng panel chi tiết (dùng chung cho Esc và nút "Bỏ chọn"). */
export function clearSelection(): void {
  useSelectionStore.getState().clear();
  useUiStore.getState().select(null);
}

/** Ghi đè yêu thích tại chỗ (faceId → bật/tắt) để thẻ đổi sao ngay mà không nạp lại lưới. */
export type FavoriteOverrides = { overrides: ReadonlyMap<number, boolean>; set(rows: readonly Pick<FaceRow, 'faceId'>[], on: boolean): void };
export const useFavoriteStore: UseBoundStore<StoreApi<FavoriteOverrides>> = create<FavoriteOverrides>()(set => ({
  overrides: new Map(),
  set: (rows, on) => set(s => { const m = new Map(s.overrides); rows.forEach(r => m.set(r.faceId, on)); return { overrides: m }; }),
}));
