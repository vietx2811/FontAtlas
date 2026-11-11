// Phím ? (Shift+/) mở lại bảng phím tắt ở bất kỳ đâu, trừ khi đang gõ chữ hoặc đã có hộp thoại/menu.
import { useEffect } from 'react';
import { ShortcutsOverlay } from './ShortcutsOverlay';
import { setShortcutsHelpOpen, useShortcutsHelpOpen } from './shortcuts';
import { isSplashOpen } from '../splash/splashState';
import { t } from '../../i18n';

export function ShortcutsHelp(): React.JSX.Element | null {
  const open = useShortcutsHelpOpen();
  useEffect(() => {
    const on = (e: KeyboardEvent) => {
      if (e.key !== '?' || e.metaKey || e.ctrlKey || e.altKey || e.repeat || e.isComposing || e.defaultPrevented || isSplashOpen()) return;
      const el = e.target as HTMLElement | null;
      if (el && (el.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName))) return;
      if (document.querySelector('[role="menu"], [role="dialog"], [role="alertdialog"], [aria-modal="true"], .fa-onb')) return;
      e.preventDefault(); setShortcutsHelpOpen(true);
    };
    document.addEventListener('keydown', on);
    return () => document.removeEventListener('keydown', on);
  }, []);
  return open ? <ShortcutsOverlay onClose={() => setShortcutsHelpOpen(false)} primary={t('tips.close')} /> : null;
}
