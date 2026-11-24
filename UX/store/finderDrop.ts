// File/thư mục kéo từ Finder đang lơ lửng trên cửa sổ (G8): đích thả sáng lên như khi kéo font từ lưới.
// Tách khỏi faceDrag vì nguồn sự kiện khác (onDragDropEvent của webview, không phải pointer events của trang).
import { create } from 'zustand';
import { targetAt, type DropTarget } from './faceDrag';

type State = { armed: boolean; over: DropTarget | null };
export const useFinderDropStore = create<State>()(() => ({ armed: false, over: null }));

/** Cập nhật theo vị trí con trỏ (px CSS); null = đã rời cửa sổ hoặc đã thả. Chỉ Dự án/Case nhận file từ Finder. */
export function hoverFinderDrop(at: { x: number; y: number } | null): void {
  if (!at) { useFinderDropStore.setState({ armed: false, over: null }); return; }
  const t = targetAt(at.x, at.y);
  const over = t && (t.kind === 'col' || t.kind === 'case') ? { ...t, valid: true } : null; // xem chính Dự án đó vẫn nhập được
  useFinderDropStore.setState({ armed: true, over });
}
