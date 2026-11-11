import { useEffect } from 'react';
import type { Api } from '../../api';
import { t } from '../../i18n';
import { useAdvancedSheet } from '../../store/advancedSheet';
import { useUiStore } from '../../store/ui';
import { filterChips } from '../main/FilterChips';
import { AdvancedSheet } from './AdvancedSheet';
import { Icon } from '../sidebar/icons';
import { Tooltip } from '../ui/Tooltip';

/** Số điều kiện bộ lọc đang bật (cùng cách đếm với tóm tắt và "Đặt lại" trong sheet). */
export function useAdvancedCount(): number {
  const filters = useUiStore(s => s.filters);
  const view = useUiStore(s => s.view);
  return filterChips(filters, [], view).length;
}

/** Phễu ở mép phải ô tìm kiếm: mở/đóng sheet Advanced, kèm số bộ lọc đang bật. */
export function AdvancedFunnel(): React.JSX.Element {
  const open = useAdvancedSheet(s => s.open);
  const setOpen = useAdvancedSheet(s => s.setOpen);
  const count = useAdvancedCount();
  const label = t('adv.open');
  return (
    <Tooltip label={count > 0 ? t('adv.funnel.tipN', { n: count }) : t('adv.funnel.tip')} shortcut="⌥⌘F">
      <button type="button" className="fa-funnel" data-testid="advanced-open" aria-label={count > 0 ? t('adv.funnel.aria', { n: count }) : label} aria-expanded={open} data-filtered={count > 0}
        onClick={e => { e.preventDefault(); e.stopPropagation(); setOpen(!open); }}>
        <Icon name="funnel" size={15} />
        {count > 0 && <span className="fa-funnel-n" data-testid="advanced-count">{count}</span>}
      </button>
    </Tooltip>
  );
}

/** Phím ⌥⌘F + sheet Advanced (nút phễu nằm trong SearchField). */
export function AdvancedFilters({ api }: { api: Api }): React.JSX.Element | null {
  const open = useAdvancedSheet(s => s.open);
  const setOpen = useAdvancedSheet(s => s.setOpen);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.metaKey && e.altKey && !e.shiftKey && (e.code === 'KeyF' || e.key.toLowerCase() === 'f' || e.key === 'ƒ')) { e.preventDefault(); useAdvancedSheet.getState().toggle(); }
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, []);
  return open ? <AdvancedSheet api={api} onClose={() => setOpen(false)} /> : null;
}
