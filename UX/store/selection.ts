// Đa chọn trong lưới/danh sách thư viện. Giữ cả FaceRow (menu cần tên, trạng thái yêu thích, đường dẫn), khoá theo `${hashHex}:${faceIndex}`.
import { create, type StoreApi, type UseBoundStore } from 'zustand';
import type { FaceRow } from '../features/library-grid/types';
import { useUiStore } from './ui';

/** ⌘A và Shift-dải không chọn quá số này (khớp trần faceKeys của truy vấn). */
export const SELECT_MAX = 2000;

export const rowKey = (r: Pick<FaceRow, 'hashHex' | 'faceIndex'>): string => `${r.hashHex}:${r.faceIndex}`;

/** Thẻ mốc của Shift-click: khoá là sự thật, index chỉ là gợi ý (thứ tự có thể đổi) và được kiểm lại trước khi dùng. */
export type SelectionAnchor = { key: string; index: number };

export interface SelectionState {
  rows: ReadonlyMap<string, FaceRow>;
  anchor: SelectionAnchor | null;
  /** Tăng mỗi lần clear: việc lấy dải/⌘A đang bay thấy epoch đổi thì bỏ kết quả. */
  epoch: number;
}
export interface SelectionActions {
  /** Chọn đúng các thẻ này. */
  replace(rows: readonly FaceRow[], anchor: SelectionAnchor | null): void;
  /** Thêm vào vùng chọn hiện có. */
  add(rows: readonly FaceRow[], anchor?: SelectionAnchor | null): void;
  /** Bỏ các thẻ này khỏi vùng chọn (không đụng phần còn lại). */
  remove(rows: readonly Pick<FaceRow, 'hashHex' | 'faceIndex'>[]): void;
  setAnchor(anchor: SelectionAnchor | null): void;
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
      set({ rows: next, anchor: { key: k, index } });
      return adding;
    },
    remove: rows => set(s => {
      const next = new Map(s.rows);
      rows.forEach(r => next.delete(rowKey(r)));
      return next.size === s.rows.size ? s : { rows: next, anchor: s.anchor && next.has(s.anchor.key) ? s.anchor : null };
    }),
    setAnchor: anchor => set(s => (s.anchor === anchor || (s.anchor === null && anchor === null) ? s : { anchor })),
    clear: () => set(s => ({ rows: s.rows.size ? new Map() : s.rows, anchor: null, epoch: s.epoch + 1 })),
  }));
}
export const useSelectionStore = createSelectionStore();

/** Bỏ chọn: xoá vùng chọn và đóng panel chi tiết (dùng chung cho Esc và nút "Bỏ chọn"). */
/** Thẻ vừa biến khỏi truy vấn (bỏ yêu thích trong Yêu thích…): bỏ khỏi vùng chọn; nếu đó là font đang xem thì chuyển sang font còn lại hoặc đóng panel. */
export function dropFromSelection(rows: readonly Pick<FaceRow, 'hashHex' | 'faceIndex'>[]): void {
  useSelectionStore.getState().remove(rows);
  const ui = useUiStore.getState();
  if (ui.selectedId && rows.some(r => rowKey(r) === ui.selectedId)) ui.select([...useSelectionStore.getState().rows.keys()].at(-1) ?? null);
}

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
