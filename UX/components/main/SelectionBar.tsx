import { useRef } from 'react';
import type { FaceRow } from '../../features/library-grid/types';
import { useUiStore } from '../../store/ui';
import { useFaceActions } from '../faces/FaceActions';
import { Icon } from '../sidebar/icons';
import { Tooltip } from '../ui/Tooltip';
import './selectionBar.css';

/** Thanh nổi trên lưới khi chọn ≥ 2 font: "N đã chọn" + hành động nhanh. */
export function SelectionBar({ rows, onClear }: { rows: readonly FaceRow[]; onClear(): void }): React.JSX.Element | null {
  const actions = useFaceActions();
  const active = useUiStore(s => s.active);
  const projectBtn = useRef<HTMLButtonElement>(null);
  if (rows.length < 2 || !actions) return null;
  const allFav = rows.every(actions.isFavorite);
  const usable = rows.filter(r => !r.missing);
  const allActive = usable.length > 0 && usable.every(r => active.has(r.hashHex));
  return (
    <div className="fa-selbar" role="toolbar" aria-label="Hành động cho các font đã chọn">
      <span className="fa-selbar-n" data-testid="selection-count">{rows.length.toLocaleString('vi-VN')} đã chọn</span>
      <span className="fa-selbar-sep" aria-hidden />
      <Tooltip label={allFav ? 'Bỏ yêu thích các font đã chọn' : 'Thêm các font đã chọn vào Yêu thích'}>
        <button type="button" onClick={() => void actions.setFavorite(rows, !allFav)}><Icon name="star" size={15} />{allFav ? 'Bỏ yêu thích' : 'Yêu thích'}</button>
      </Tooltip>
      <Tooltip label="Thêm vào một dự án có sẵn hoặc tạo dự án mới">
        <button ref={projectBtn} type="button" aria-haspopup="menu" onClick={e => actions.openProjectMenu(rows, { rect: e.currentTarget.getBoundingClientRect() }, { returnFocus: e.currentTarget })}><Icon name="folder" size={15} />Thêm vào dự án</button>
      </Tooltip>
      <Tooltip label={usable.length === 0 ? 'File không còn trên máy' : allActive ? 'Tắt kích hoạt các font đã chọn' : 'Kích hoạt các font đã chọn trong hệ thống'}>
        <button type="button" aria-disabled={usable.length === 0 || undefined} onClick={() => { if (usable.length) void actions.setActive(usable, !allActive); }}><Icon name="power" size={15} />{allActive ? 'Tắt kích hoạt' : 'Kích hoạt'}</button>
      </Tooltip>
      <span className="fa-selbar-sep" aria-hidden />
      <Tooltip label="Bỏ chọn" shortcut="Esc">
        <button type="button" onClick={onClear}>Bỏ chọn</button>
      </Tooltip>
    </div>
  );
}
