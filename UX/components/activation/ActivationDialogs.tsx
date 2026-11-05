import { useState } from 'react';
import type { ConflictDto } from '../../api';
import { Modal } from '../Modal';

const OWNER: Record<ConflictDto['owner'], string> = {
  system: 'hệ thống macOS', user_installed: 'font bạn đã cài', adobe_fonts: 'Adobe Fonts', right_font: 'RightFont', font_base: 'FontBase',
  font_atlas: 'một file khác trong thư viện', other: 'một ứng dụng khác',
};

type ConflictProps = { conflicts: ConflictDto[]; total: number; onCancel(): void; onConfirm(includeConflicts: boolean): void };

/** D3 (UX 03 §4.3): trùng tên PostScript với font đang có. Mặc định bỏ qua các font trùng. */
export function ConflictDialog({ conflicts, total, onCancel, onConfirm }: ConflictProps): React.JSX.Element {
  const [include, setInclude] = useState(false);
  const n = conflicts.length;
  const blocked = conflicts.some(c => c.owner === 'system');
  return (
    <Modal title={`${n} font trùng tên với font đang có trên máy`} onClose={onCancel}>
      <ul className="fa-d3-list">
        {conflicts.map(c => (
          <li key={`${c.postscriptName}${c.existingPath}`}>
            <strong>{c.postscriptName}</strong> đang có: {OWNER[c.owner]} <span className="fa-d4-count">({c.existingPath})</span>
          </li>
        ))}
      </ul>
      <p>App thiết kế có thể dùng nhầm bản khi hai bản cùng tên.</p>
      <label className="fa-radio"><input type="radio" name="d3" checked={!include} onChange={() => setInclude(false)} />
        {total === n ? `Bỏ qua ${n === 1 ? 'font này' : `${n} font này`}` : `Bỏ qua ${n} font này, kích hoạt ${total - n} font còn lại`}</label>
      <label className="fa-radio"><input type="radio" name="d3" checked={include} disabled={blocked} onChange={() => setInclude(true)} />
        Vẫn kích hoạt cả {total} font{blocked && <em> (font hệ thống luôn được ưu tiên nên không thể kích hoạt thêm bản trùng)</em>}</label>
      <div className="fa-modal-actions">
        <button type="button" onClick={onCancel}>Huỷ</button>
        <button type="button" className="primary" onClick={() => onConfirm(include)}>Tiếp tục</button>
      </div>
    </Modal>
  );
}

type HeldProps = { collectionCount: number; onCancel(): void; onConfirm(): void };
export function HeldDialog({ collectionCount, onCancel, onConfirm }: HeldProps): React.JSX.Element {
  return (
    <Modal title={`Font này đang được giữ bởi ${collectionCount} dự án`} onClose={onCancel}>
      <p>Tắt luôn sẽ gỡ nó khỏi cả các dự án đang kích hoạt. Font vẫn nằm trong dự án, chỉ không còn được bật.</p>
      <div className="fa-modal-actions">
        <button type="button" onClick={onCancel}>Huỷ</button>
        <button type="button" className="primary" onClick={onConfirm}>Tắt luôn</button>
      </div>
    </Modal>
  );
}
