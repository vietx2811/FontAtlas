import { useEffect } from 'react';
import { useUiStore } from '../../store/ui';

/** ⌘, (hoặc Ctrl+,) mở/đóng Cài đặt từ bất kỳ đâu. */
export function SettingsShortcut(): null {
  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === ',') { e.preventDefault(); useUiStore.getState().toggleSettings(); }
    };
    document.addEventListener('keydown', on);
    return () => document.removeEventListener('keydown', on);
  }, []);
  return null;
}
