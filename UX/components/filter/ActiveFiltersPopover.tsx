import { useEffect, useRef } from 'react';
import { t } from '../../i18n';
import type { Group } from './groupFilters';
import './filter.css';

/** "Đang lọc": mỗi nhóm một hàng — × từng giá trị, Bỏ nhóm; Xoá tất cả. Esc hoặc bấm ra ngoài thì đóng. */
export function ActiveFiltersPopover({ groups, onRemoveChip, onRemoveGroup, onClearAll, onClose }: {
  groups: Group[]; onRemoveChip(g: Group, chipId: string): void; onRemoveGroup(g: Group): void; onClearAll(): void; onClose(): void;
}): React.JSX.Element {
  const root = useRef<HTMLDivElement>(null);
  useEffect(() => {
    root.current?.focus();
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') { e.stopPropagation(); onClose(); } };
    const onDown = (e: PointerEvent) => { if (root.current && !root.current.contains(e.target as Node) && !(e.target as Element).closest?.('[data-testid=filter-summary]')) onClose(); };
    document.addEventListener('keydown', onKey, true);
    document.addEventListener('pointerdown', onDown, true);
    return () => { document.removeEventListener('keydown', onKey, true); document.removeEventListener('pointerdown', onDown, true); };
  }, [onClose]);
  return (
    <div ref={root} className="fa-fpop" role="dialog" aria-label="Đang lọc" tabIndex={-1} data-testid="active-filters">
      {groups.map(g => (
        <div key={g.id} className="fa-fpop-row" data-group={g.id}>
          <span className="fa-fpop-label">{g.label}</span>
          <span className="fa-fpop-chips">
            {g.chips.map(c => (
              <span key={c.id} className="fa-fpop-chip">{c.label}
                <button type="button" aria-label={t('fgroup.removeValue', { label: c.label })} onClick={() => onRemoveChip(g, c.id)}>×</button>
              </span>
            ))}
          </span>
          <button type="button" className="fa-fpop-drop" onClick={() => onRemoveGroup(g)}>{t('fgroup.remove')}</button>
        </div>
      ))}
      <div className="fa-fpop-foot"><button type="button" onClick={onClearAll}>{t('fgroup.clearAll')}</button></div>
    </div>
  );
}
