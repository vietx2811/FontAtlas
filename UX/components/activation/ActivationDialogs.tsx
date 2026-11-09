import { useMemo, useState } from 'react';
import { t } from '../../i18n';
import { Modal } from '../Modal';
import { NoteMark } from '../ui/noteMark';
import { groupByFamily, groupNote, type ConflictItem } from './conflictModel';
import './conflictDialog.css';

type ConflictProps = { items: ConflictItem[]; total: number; onCancel(): void; onConfirm(includeConflicts: boolean): void };

/**
 * D3 (UX 03 §4.3): trùng tên PostScript với font đang có. Gộp theo family (một dòng mỗi family, mở ra xem kiểu); ai cài và đường dẫn
 * chỉ nằm trong tooltip của dấu "!". Mặc định bỏ qua các font trùng. Hộp cao tối đa 70% cửa sổ, danh sách cuộn bên trong.
 */
export function ConflictDialog({ items, total, onCancel, onConfirm }: ConflictProps): React.JSX.Element {
  const [include, setInclude] = useState(false);
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set());
  const groups = useMemo(() => groupByFamily(items), [items]);
  const n = items.length;
  const blocked = items.some(i => i.conflicts.some(c => c.owner === 'system'));
  const flip = (f: string) => setOpen(s => { const x = new Set(s); if (x.has(f)) x.delete(f); else x.add(f); return x; });
  return (
    <Modal title={t('act.conflict.title', { n })} onClose={onCancel} className="fa-conf">
      <p>{t('act.conflict.why')}</p>
      <ul className="fa-conf-list" data-testid="conflict-list">
        {groups.map(g => {
          const isOpen = open.has(g.family);
          return (
            <li key={g.family} className="fa-note-host">
              <button type="button" className="fa-conf-row" aria-expanded={isOpen} aria-label={t(isOpen ? 'act.conflict.collapse' : 'act.conflict.expand', { family: g.family })} onClick={() => flip(g.family)}>
                <span className="chev" aria-hidden data-open={isOpen} />
                <span className="nm">{t('act.conflict.family', { family: g.family, n: g.styles.length })}</span>
              </button>
              <NoteMark text={t('act.conflict.where', { where: groupNote(g) })} kind="info" inline />
              {isOpen && <ul className="fa-conf-styles">{g.styles.map(s => <li key={s}>{s}</li>)}</ul>}
            </li>
          );
        })}
      </ul>
      <label className="fa-radio"><input type="radio" name="d3" checked={!include} onChange={() => setInclude(false)} />
        {total === n ? t('act.conflict.skip') : t('act.conflict.skip.rest', { n: total - n })}</label>
      <label className="fa-radio"><input type="radio" name="d3" checked={include} disabled={blocked} onChange={() => setInclude(true)} />
        {total === n ? t('act.conflict.force') : t('act.conflict.force.all', { total })}{blocked && <em> ({t('act.conflict.blocked')})</em>}</label>
      <div className="fa-modal-actions">
        <button type="button" onClick={onCancel}>{t('act.conflict.cancel')}</button>
        <button type="button" className="primary" onClick={() => onConfirm(include)}>{t('act.conflict.go')}</button>
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
