import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { t } from '../../i18n';
import type { Group } from './groupFilters';
import './filter.css';

/** "Đang lọc": mỗi nhóm một hàng — × từng giá trị, Bỏ nhóm; Xoá tất cả. Esc hoặc bấm ra ngoài thì đóng. */
export function ActiveFiltersPopover({ anchor, groups, onRemoveChip, onRemoveGroup, onClearAll, onClose }: {
  /** Phần tử để canh vị trí; popover render ra body (không nằm trong <label> của ô tìm, nên bấm trong popover không kéo focus về ô nhập). */
  anchor: HTMLElement | null; groups: Group[]; onRemoveChip(g: Group, chipId: string): void; onRemoveGroup(g: Group): void; onClearAll(): void; onClose(): void;
}): React.JSX.Element {
  const root = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ left: number; top: number } | null>(null);
  useLayoutEffect(() => {
    const place = () => { const r = anchor?.getBoundingClientRect(); if (r) setPos({ left: Math.round(r.left), top: Math.round(r.bottom + 6) }); };
    place(); window.addEventListener('resize', place);
    return () => window.removeEventListener('resize', place);
  }, [anchor]);
  useEffect(() => {
    root.current?.focus();
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') { e.stopImmediatePropagation(); e.preventDefault(); onClose(); } };
    const onDown = (e: PointerEvent) => { if (root.current && !root.current.contains(e.target as Node) && !(e.target as Element).closest?.('[data-testid=filter-summary]')) onClose(); };
    document.addEventListener('keydown', onKey, true);
    document.addEventListener('pointerdown', onDown, true);
    return () => { document.removeEventListener('keydown', onKey, true); document.removeEventListener('pointerdown', onDown, true); };
  }, [onClose]);
  return createPortal(
    <div ref={root} className="fa-fpop" role="dialog" aria-label={t('fgroup.title')} tabIndex={-1} data-testid="active-filters" style={pos ? { left: pos.left, top: pos.top } : undefined}>
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
    </div>,
    document.body,
  );
}
