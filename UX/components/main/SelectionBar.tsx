import { useRef } from 'react';
import type { FaceRow } from '../../features/library-grid/types';
import { clearSelection, useFavoriteStore } from '../../store/selection';
import { useUiStore } from '../../store/ui';
import { t } from '../../i18n';
import { useFaceActions } from '../faces/FaceActions';
import { Icon } from '../sidebar/icons';
import { Tooltip } from '../ui/Tooltip';
import './selectionBar.css';

/** Thanh trong hàng dưới topbar khi chọn ≥ 2 font: "N đã chọn" + hành động nhanh (không đè lên lưới). */
export function SelectionBar({ rows }: { rows: readonly FaceRow[] }): React.JSX.Element | null {
  const actions = useFaceActions();
  const active = useUiStore(s => s.active);
  const available = useUiStore(s => s.available);
  useFavoriteStore(s => s.overrides); // đổi yêu thích → nhãn Yêu thích/Bỏ yêu thích vẽ lại
  const projectBtn = useRef<HTMLButtonElement>(null);
  if (rows.length < 2 || !actions) return null;
  const allFav = rows.every(actions.isFavorite);
  const usable = rows.filter(r => !r.missing);
  // "Đã bật" gồm cả font có sẵn trên máy nhờ bản giống hệt của bên khác: Kích hoạt không còn gì để làm với chúng.
  const allActive = usable.length > 0 && usable.every(r => active.has(r.hashHex) || available.has(r.hashHex));
  return (
    <div className="fa-selbar" role="toolbar" aria-label={t('sel.group')}>
      <span className="fa-selbar-n" data-testid="selection-count">{t('sel.count', { n: rows.length.toLocaleString('vi-VN') })}</span>
      <span className="fa-selbar-sep" aria-hidden />
      <Tooltip label={t(allFav ? 'sel.unfav.tip' : 'sel.fav.tip')}>
        <button type="button" onClick={() => void actions.setFavorite(rows, !allFav)}><Icon name="star" size={15} />{t(allFav ? 'sel.unfav' : 'sel.fav')}</button>
      </Tooltip>
      <Tooltip label={t('sel.project.tip')}>
        <button ref={projectBtn} type="button" aria-haspopup="menu" onClick={e => actions.openProjectMenu(rows, { rect: e.currentTarget.getBoundingClientRect() }, { returnFocus: e.currentTarget })}><Icon name="folder" size={15} />{t('sel.project')}</button>
      </Tooltip>
      <Tooltip label={t(usable.length === 0 ? 'sel.missing.tip' : allActive ? 'sel.deactivate.tip' : 'sel.activate.tip')}>
        <button type="button" data-testid="selbar-activate" aria-disabled={usable.length === 0 || undefined} onClick={() => { if (usable.length) void actions.setActive(usable, !allActive); }}><Icon name="power" size={15} />{t(allActive ? 'sel.deactivate' : 'sel.activate')}</button>
      </Tooltip>
      <span className="fa-selbar-sep" aria-hidden />
      <Tooltip label={t('sel.clear')} shortcut="Esc">
        <button type="button" onClick={clearSelection}>{t('sel.clear')}</button>
      </Tooltip>
    </div>
  );
}
