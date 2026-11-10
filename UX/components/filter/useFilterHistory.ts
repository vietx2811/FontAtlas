import { useCallback, useEffect, useRef, useState } from 'react';
import { useUiStore, type LibraryFilters } from '../../store/ui';

export const FILTER_HISTORY_LIMIT = 20;
export const COALESCE_MS = 400;
const RANGE_KEYS = new Set(['rawAxisRanges', 'weightClass', 'widthClass']);
/** Chỉ các khoá thanh trượt đổi (các khoá khác giữ nguyên): các lần đổi liên tiếp được gộp một bước hoàn tác. */
export function rangeOnly(a: LibraryFilters, b: LibraryFilters): boolean {
  const x = a as Record<string, unknown>; const y = b as Record<string, unknown>;
  const changed = [...new Set([...Object.keys(x), ...Object.keys(y)])].filter(k => JSON.stringify(x[k]) !== JSON.stringify(y[k]));
  return changed.length > 0 && changed.every(k => RANGE_KEYS.has(k));
}

/** Ô nhập chữ đang focus có undo riêng: ⌘Z ở đó không được bị bộ lọc cướp. */
export function isEditableTarget(el: EventTarget | null): boolean {
  if (!(el instanceof HTMLElement)) return false;
  if (el.isContentEditable) return true;
  if (el instanceof HTMLTextAreaElement) return true;
  if (el instanceof HTMLInputElement) return !['checkbox', 'radio', 'button', 'range', 'submit', 'reset'].includes(el.type);
  return false;
}

/**
 * Ngăn xếp hoàn tác cho `filters` của store (tối đa 20 bước). Mọi thay đổi `filters` — từ bất kỳ nơi nào — được ghi lại;
 * `undo()` trả về trạng thái trước mà không tự ghi thêm. ⌘Z toàn cục bỏ qua khi focus nằm trong ô nhập chữ.
 * Gọi MỘT lần ở nơi sở hữu bộ lọc (SearchField).
 */
export function useFilterHistory(): { canUndo: boolean; undo(): boolean; clear(): void } {
  const filters = useUiStore(s => s.filters);
  const view = useUiStore(s => s.view);
  const familyKey = useUiStore(s => s.familyScope?.name ?? '');
  const lastAt = useRef(0);
  const lastRange = useRef(false);
  const stack = useRef<LibraryFilters[]>([]);
  const prev = useRef(filters);
  const skip = useRef(false);
  const [size, setSize] = useState(0);

  useEffect(() => {
    if (filters === prev.current) return;
    const now = Date.now();
    if (skip.current) skip.current = false;
    else if (rangeOnly(prev.current, filters) && lastRange.current && now - lastAt.current <= COALESCE_MS && stack.current.length) { /* cùng một lần chỉnh trục: giữ bước đầu */ }
    else { stack.current.push(prev.current); if (stack.current.length > FILTER_HISTORY_LIMIT) stack.current.shift(); }
    lastAt.current = now; lastRange.current = rangeOnly(prev.current, filters);
    prev.current = filters;
    setSize(stack.current.length);
  }, [filters]);

  const undo = useCallback((): boolean => {
    const before = stack.current.pop();
    if (!before) return false;
    skip.current = true;
    useUiStore.getState().setFilters(before);
    setSize(stack.current.length);
    return true;
  }, []);
  const clear = useCallback(() => { stack.current = []; setSize(0); }, []);
  // Đổi view/family: bộ lọc cũ không còn thuộc ngữ cảnh này.
  useEffect(() => { stack.current = []; setSize(0); }, [view, familyKey]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.shiftKey || e.altKey || e.key.toLowerCase() !== 'z') return;
      if (isEditableTarget(e.target) || isEditableTarget(document.activeElement)) return;
      if (undo()) e.preventDefault();
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [undo]);

  return { canUndo: size > 0, undo, clear };
}
