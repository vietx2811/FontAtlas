// Kéo font từ lưới thư viện thả vào mục thanh bên (G1). Kéo bằng con trỏ (pointer events), không dùng HTML5 DnD:
// ổn định trong WebView của Tauri (nơi sự kiện kéo-thả file của hệ điều hành có thể chặn DnD của trang) và cho phép
// vẽ "con trỏ báo số font" tuỳ ý. Điểm thả là phần tử có `data-drop-col` / `-case` / `-active` / `-tag` (xem NavItem và Sidebar).
import { create } from 'zustand';
import type { FaceRow } from '../features/library-grid/types';
import { useSelectionStore, rowKey } from './selection';

export type DropKind = 'col' | 'case' | 'active' | 'tag' | 'compare' | 'role';
export type DropTarget = { kind: DropKind; id: number; /** Chỉ `role`: tên vai (khoá lưu) của ô nhận. */ role?: string; name: string; /** false = đích không hợp lệ (đang xem chính dự án đó). */ valid: boolean };
/** `moveFrom` = id Dự án đang xem làm nguồn kéo (null: kéo từ nơi không phải Dự án, không có gì để chuyển khỏi); `move` = đang giữ ⌘ (chuyển thay vì thêm). */
export type FaceDragState = { rows: readonly FaceRow[]; x: number; y: number; over: DropTarget | null; moveFrom: number | null; move: boolean; /** Kéo từ một vai trong Pairing Desk (đổi chỗ font giữa hai vai). */ fromRole?: string };
type Store = { drag: FaceDragState | null };
export const useFaceDragStore = create<Store>()(() => ({ drag: null }));

/** `moveFrom`: Dự án nguồn cần gỡ font khỏi khi người dùng giữ ⌘ lúc thả (chuyển); null = chỉ thêm. */
export type DropHandler = (rows: readonly FaceRow[], target: DropTarget, opts: { moveFrom: number | null; fromRole?: string }) => void;
/** Ngăn xếp nơi nhận: màn nào đang trên cùng (Pairing Desk phủ lên Library) nhận lượt thả; gỡ đăng ký thì màn dưới nhận lại. */
const stack: Array<{ h: DropHandler; source: () => number | null }> = [];
/** Nơi nhận thao tác thả (Library đăng ký, có `api`) và cách biết Dự án đang xem (nguồn kéo). Trả hàm gỡ đăng ký. */
export function registerFaceDropHandler(h: DropHandler, source: () => number | null = () => null): () => void { const e = { h, source }; stack.push(e); return () => { const i = stack.indexOf(e); if (i >= 0) stack.splice(i, 1); }; }

/** Quá ngưỡng này (px) mới coi là kéo; dưới ngưỡng vẫn là bấm chọn bình thường. */
export const DRAG_THRESHOLD = 5;

export function targetAt(x: number, y: number, doc: Document = document): DropTarget | null {
  const el = doc.elementFromPoint(x, y)?.closest<HTMLElement>('[data-drop-col], [data-drop-case], [data-drop-active], [data-drop-tag], [data-drop-compare], [data-drop-role]');
  if (!el) return null;
  const d = el.dataset;
  const kind: DropKind = d.dropCase !== undefined ? 'case' : d.dropActive !== undefined ? 'active' : d.dropTag !== undefined ? 'tag' : d.dropCompare !== undefined ? 'compare' : d.dropRole !== undefined ? 'role' : 'col';
  if (kind === 'role') return { kind, id: 0, role: d.dropRole ?? '', name: d.dropName ?? '', valid: d.dropHere !== 'true' };
  const raw = kind === 'case' ? d.dropCase : kind === 'active' ? d.dropActive : kind === 'tag' ? d.dropTag : kind === 'compare' ? d.dropCompare : d.dropCol;
  const id = Number(raw);
  if (!Number.isFinite(id)) return null;
  return { kind, id, name: el.dataset.dropName ?? '', valid: el.dataset.dropHere !== 'true' };
}

let cancel: (() => void) | null = null;
/** Hủy lượt kéo đang chạy (Esc, mất focus, hoặc gỡ màn hình). */
export function cancelFaceDrag(): void { cancel?.(); }

/** Gọi từ `onPointerDown` của thẻ font. Chuột trái / bút / chạm; bỏ qua khi bấm vào nút con (chấm bật, sao…). */
export type BeginOpts = {
  /** Kéo từ vai này trong Desk. */ fromRole?: string;
  /** Chỉ mang đúng thẻ này, bỏ qua vùng chọn của lưới (ứng viên/vai trong Desk). */ solo?: boolean;
  /** Điểm bấm chính nó là một nút (hàng vai): vẫn cho kéo, chỉ bỏ qua nút con. */ ownButton?: boolean;
};
export function beginFaceDrag(e: { button: number; clientX: number; clientY: number; pointerId: number; target: EventTarget | null; currentTarget?: EventTarget | null }, row: FaceRow, o: BeginOpts = {}): void {
  if (e.button !== 0 || cancel) return;
  const hit = (e.target as Element | null)?.closest?.('button, input, a, select, textarea');
  if (hit && !(o.ownButton && hit === e.currentTarget)) return;
  const sx = e.clientX, sy = e.clientY;
  let dragging = false;
  let rows: readonly FaceRow[] = [];
  let lastX = sx, lastY = sy, meta = false;
  const moveFrom = o.solo || o.fromRole ? null : (stack.at(-1)?.source() ?? null);
  const set = (x: number, y: number) => useFaceDragStore.setState({ drag: { rows, x, y, over: targetAt(x, y), moveFrom, move: meta && moveFrom !== null, fromRole: o.fromRole } });
  const onMove = (ev: PointerEvent) => {
    if (!dragging) {
      if (Math.hypot(ev.clientX - sx, ev.clientY - sy) < DRAG_THRESHOLD) return;
      // Kéo thẻ đang nằm trong vùng chọn → kéo cả vùng chọn; kéo thẻ ngoài vùng chọn → chỉ thẻ đó.
      const sel = useSelectionStore.getState().rows;
      rows = !o.solo && sel.has(rowKey(row)) ? [...sel.values()] : [row];
      dragging = true;
      document.body.dataset.faceDragging = 'true';
    }
    meta = ev.metaKey === true; lastX = ev.clientX; lastY = ev.clientY;
    set(ev.clientX, ev.clientY);
  };
  const onMeta = (ev: KeyboardEvent) => { if (dragging && ev.key === 'Meta') { meta = ev.type === 'keydown'; set(lastX, lastY); } };
  const end = () => {
    window.removeEventListener('pointermove', onMove, true);
    window.removeEventListener('pointerup', onUp, true);
    window.removeEventListener('pointercancel', abort, true);
    window.removeEventListener('keydown', onKey, true);
    window.removeEventListener('keydown', onMeta);
    window.removeEventListener('keyup', onMeta);
    window.removeEventListener('blur', abort);
    delete document.body.dataset.faceDragging;
    useFaceDragStore.setState({ drag: null });
    cancel = null;
  };
  // Sau khi thả, trình duyệt vẫn bắn `click` lên thẻ: nuốt một lần để không đổi vùng chọn.
  const swallowClick = () => {
    const eat = (ev: Event) => { ev.stopPropagation(); ev.preventDefault(); };
    window.addEventListener('click', eat, { capture: true, once: true });
    setTimeout(() => window.removeEventListener('click', eat, true), 0);
  };
  const onUp = (ev: PointerEvent) => {
    const was = dragging; const target = was ? targetAt(ev.clientX, ev.clientY) : null; const picked = rows;
    const mv = (ev.metaKey === true || meta) && moveFrom !== null ? moveFrom : null;
    end();
    if (!was) return;
    swallowClick();
    const top = stack.at(-1);
    if (target?.valid && top) top.h(picked, target, { moveFrom: mv, fromRole: o.fromRole });
  };
  const abort = () => { const was = dragging; end(); if (was) swallowClick(); };
  const onKey = (ev: KeyboardEvent) => { if (ev.key === 'Escape' && dragging) { ev.stopPropagation(); ev.preventDefault(); abort(); } };
  cancel = abort;
  window.addEventListener('pointermove', onMove, true);
  window.addEventListener('pointerup', onUp, true);
  window.addEventListener('pointercancel', abort, true);
  window.addEventListener('keydown', onKey, true);
  window.addEventListener('keydown', onMeta);
  window.addEventListener('keyup', onMeta);
  window.addEventListener('blur', abort);
}
