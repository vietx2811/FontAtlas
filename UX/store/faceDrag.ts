// Kéo font từ lưới thư viện thả vào mục thanh bên (G1). Kéo bằng con trỏ (pointer events), không dùng HTML5 DnD:
// ổn định trong WebView của Tauri (nơi sự kiện kéo-thả file của hệ điều hành có thể chặn DnD của trang) và cho phép
// vẽ "con trỏ báo số font" tuỳ ý. Điểm thả là phần tử có `data-drop-col` / `-case` / `-active` / `-tag` (xem NavItem và Sidebar).
import { create } from 'zustand';
import type { FaceRow } from '../features/library-grid/types';
import { useSelectionStore, rowKey } from './selection';

export type DropKind = 'col' | 'case' | 'active' | 'tag';
export type DropTarget = { kind: DropKind; id: number; name: string; /** false = đích không hợp lệ (đang xem chính dự án đó). */ valid: boolean };
export type FaceDragState = { rows: readonly FaceRow[]; x: number; y: number; over: DropTarget | null };
type Store = { drag: FaceDragState | null };
export const useFaceDragStore = create<Store>()(() => ({ drag: null }));

export type DropHandler = (rows: readonly FaceRow[], target: DropTarget) => void;
let handler: DropHandler | null = null;
/** Nơi nhận thao tác thả (Library đăng ký, có `api`). Trả hàm gỡ đăng ký. */
export function registerFaceDropHandler(h: DropHandler): () => void { handler = h; return () => { if (handler === h) handler = null; }; }

/** Quá ngưỡng này (px) mới coi là kéo; dưới ngưỡng vẫn là bấm chọn bình thường. */
export const DRAG_THRESHOLD = 5;

export function targetAt(x: number, y: number, doc: Document = document): DropTarget | null {
  const el = doc.elementFromPoint(x, y)?.closest<HTMLElement>('[data-drop-col], [data-drop-case], [data-drop-active], [data-drop-tag]');
  if (!el) return null;
  const kind: DropKind = el.dataset.dropCase !== undefined ? 'case' : el.dataset.dropActive !== undefined ? 'active' : el.dataset.dropTag !== undefined ? 'tag' : 'col';
  const raw = kind === 'case' ? el.dataset.dropCase : kind === 'active' ? el.dataset.dropActive : kind === 'tag' ? el.dataset.dropTag : el.dataset.dropCol;
  const id = Number(raw);
  if (!Number.isFinite(id)) return null;
  return { kind, id, name: el.dataset.dropName ?? '', valid: el.dataset.dropHere !== 'true' };
}

let cancel: (() => void) | null = null;
/** Hủy lượt kéo đang chạy (Esc, mất focus, hoặc gỡ màn hình). */
export function cancelFaceDrag(): void { cancel?.(); }

/** Gọi từ `onPointerDown` của thẻ font. Chuột trái / bút / chạm; bỏ qua khi bấm vào nút con (chấm bật, sao…). */
export function beginFaceDrag(e: { button: number; clientX: number; clientY: number; pointerId: number; target: EventTarget | null }, row: FaceRow): void {
  if (e.button !== 0 || cancel) return;
  if ((e.target as Element | null)?.closest?.('button, input, a, select, textarea')) return;
  const sx = e.clientX, sy = e.clientY;
  let dragging = false;
  let rows: readonly FaceRow[] = [];
  const set = (x: number, y: number) => useFaceDragStore.setState({ drag: { rows, x, y, over: targetAt(x, y) } });
  const onMove = (ev: PointerEvent) => {
    if (!dragging) {
      if (Math.hypot(ev.clientX - sx, ev.clientY - sy) < DRAG_THRESHOLD) return;
      // Kéo thẻ đang nằm trong vùng chọn → kéo cả vùng chọn; kéo thẻ ngoài vùng chọn → chỉ thẻ đó.
      const sel = useSelectionStore.getState().rows;
      rows = sel.has(rowKey(row)) ? [...sel.values()] : [row];
      dragging = true;
      document.body.dataset.faceDragging = 'true';
    }
    set(ev.clientX, ev.clientY);
  };
  const end = () => {
    window.removeEventListener('pointermove', onMove, true);
    window.removeEventListener('pointerup', onUp, true);
    window.removeEventListener('pointercancel', abort, true);
    window.removeEventListener('keydown', onKey, true);
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
    end();
    if (!was) return;
    swallowClick();
    if (target?.valid && handler) handler(picked, target);
  };
  const abort = () => { const was = dragging; end(); if (was) swallowClick(); };
  const onKey = (ev: KeyboardEvent) => { if (ev.key === 'Escape' && dragging) { ev.stopPropagation(); ev.preventDefault(); abort(); } };
  cancel = abort;
  window.addEventListener('pointermove', onMove, true);
  window.addEventListener('pointerup', onUp, true);
  window.addEventListener('pointercancel', abort, true);
  window.addEventListener('keydown', onKey, true);
  window.addEventListener('blur', abort);
}
