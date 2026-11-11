import { useEffect } from 'react';
import { useUiStore } from '../../store/ui';
import { shortcutBlocked } from './TopBarActions';

/** ⌥G: bật/tắt nhóm theo Family khi không gõ chữ / IME / hộp thoại. Đăng ký ở Library (luôn dựng cùng lưới), không phụ thuộc popover Hiển thị có đang dựng hay không. */
export function useGroupShortcut(): void {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!e.altKey || e.metaKey || e.ctrlKey || e.shiftKey || e.defaultPrevented || e.isComposing || e.repeat || e.code !== 'KeyG') return;
      if (shortcutBlocked(e)) return;
      e.preventDefault(); useUiStore.getState().setGroupByFamily(!useUiStore.getState().sort.group);
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, []);
}
