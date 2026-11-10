import { useEffect } from 'react';
import type { Api } from '../../api';
import { t } from '../../i18n';
import { useAdvancedSheet } from '../../store/advancedSheet';
import { useUiStore } from '../../store/ui';
import { filterChips } from '../main/FilterChips';
import { AdvancedSheet } from './AdvancedSheet';

/** Nút "Advanced" trên Top Bar + phím ⌥⌘F + sheet. */
export function AdvancedFilters({ api }: { api: Api }): React.JSX.Element {
  const open = useAdvancedSheet(s => s.open);
  const setOpen = useAdvancedSheet(s => s.setOpen);
  const filters = useUiStore(s => s.filters);
  const view = useUiStore(s => s.view);
  const count = filterChips(filters, [], view).length; // cùng cách đếm với tóm tắt và "Đặt lại" trong sheet
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey && e.altKey && !e.shiftKey && (e.code === 'KeyF' || e.key.toLowerCase() === 'f' || e.key === 'ƒ')) { e.preventDefault(); useAdvancedSheet.getState().toggle(); }
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, []);
  return <>
    <button type="button" className="fa-vi-toggle" data-testid="advanced-open" aria-expanded={open} aria-pressed={open} data-selected={count > 0} onClick={() => setOpen(!open)}>{t('adv.open')}</button>
    {open && <AdvancedSheet api={api} onClose={() => setOpen(false)} />}
  </>;
}
