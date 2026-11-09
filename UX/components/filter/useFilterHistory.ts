import { useCallback, useEffect, useRef, useState } from 'react';
import { useUiStore, type LibraryFilters } from '../../store/ui';

export const FILTER_HISTORY_LIMIT = 20;

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
  const stack = useRef<LibraryFilters[]>([]);
  const prev = useRef(filters);
  const skip = useRef(false);
  const [size, setSize] = useState(0);

  useEffect(() => {
    if (filters === prev.current) return;
    if (skip.current) skip.current = false;
    else { stack.current.push(prev.current); if (stack.current.length > FILTER_HISTORY_LIMIT) stack.current.shift(); }
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
