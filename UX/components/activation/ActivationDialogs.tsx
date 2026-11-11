import { useMemo, useState } from 'react';
import { t, tn } from '../../i18n';
import { Modal } from '../Modal';
import { NoteMark } from '../ui/noteMark';
import { TermTip } from '../ui/terms';
import { groupByFamily, groupNote, replaceAdvice, type ConflictItem } from './conflictModel';
import './conflictDialog.css';

type ConflictProps = {
  items: ConflictItem[]; total: number; onCancel(): void; onConfirm(includeConflicts: boolean): void;
  /** "Thay bằng bản này": tắt bản cũ (theo ma trận) rồi bật bản của Font Atlas. Thiếu → không hiện lựa chọn. */
  onReplace?(): void;
  /** "Thử lại" sau khi người dùng tự tắt bản cũ trong app đó. */
  onRetry?(): void;
  /** "Hiện trong Finder" cho file bản cũ (font hệ thống / thư mục font: không bao giờ đụng). */
  onReveal?(path: string): void;
};
type Choice = 'skip' | 'both' | 'replace';

/** Thay `{tên}` trong chuỗi i18n bằng phần tử React (vd TermTip), giữ nguyên phần chữ còn lại. */
function fillSlots(text: string, slots: Record<string, React.ReactNode>): React.ReactNode[] {
  return text.split(/(\{\w+\})/).map((part, i) => {
    const m = /^\{(\w+)\}$/.exec(part);
    return m && m[1] in slots ? <span key={i}>{slots[m[1]]}</span> : part;
  });
}

/**
 * D3 (UX 03 §4.3): trùng tên PostScript với font đang có. Gộp theo family (một dòng mỗi family, mở ra xem kiểu); ai cài và đường dẫn
 * chỉ nằm trong tooltip của dấu "!". Mặc định bỏ qua các font trùng. Hộp cao tối đa 70% cửa sổ, danh sách cuộn bên trong.
 */
export function ConflictDialog({ items, total, onCancel, onConfirm, onReplace, onRetry, onReveal }: ConflictProps): React.JSX.Element {
  const advice = useMemo(() => replaceAdvice(items), [items]);
  const canReplace = advice.tier === 'replace' && !!onReplace;
  // chọn sẵn "Thay bằng bản này" khi được phép và chỉ có app đã nhận diện; có app lạ → giữ "bỏ qua" + xác nhận mạnh
  const [choice, setChoice] = useState<Choice>(() => (canReplace && !advice.unknownApp ? 'replace' : 'skip'));
  const [understood, setUnderstood] = useState(false);
  const appName = advice.apps.join(' / ');
  const other = t('act.replace.app.other');
  // mọi app (đã nhận diện và "một ứng dụng khác") đều được kể tên trong cảnh báo; có bất kỳ app lạ nào → xác nhận mạnh
  const names = [...advice.apps, ...(advice.unknownApp ? [other] : [])];
  const guideApp = names.join(' / ');
  const strong = canReplace && advice.unknownApp;
  const warnText = advice.apps.length === 0 ? t('act.replace.warn.other') : t('act.replace.warn', { app: guideApp });
  const warnId = 'fa-replace-warn';
  const goDisabled = choice === 'replace' && strong && !understood;
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
      <label className="fa-radio"><input type="radio" name="d3" checked={choice === 'skip'} onChange={() => setChoice('skip')} />
        {total === n ? t('act.conflict.skip') : t('act.conflict.skip.rest', { n: total - n })}</label>
      {canReplace && (
        <label className="fa-radio"><input type="radio" name="d3" checked={choice === 'replace'} onChange={() => setChoice('replace')} aria-describedby={warnId} />
          {t('act.replace.option')} <span className="fa-conf-gloss">· {t('act.replace.option.gloss')}</span></label>
      )}
      <label className="fa-radio"><input type="radio" name="d3" checked={choice === 'both'} disabled={blocked} onChange={() => setChoice('both')} />
        {total === n ? t('act.conflict.force') : t('act.conflict.force.all', { total })}{blocked && <em> ({t('act.conflict.blocked')})</em>}</label>
      {canReplace && (
        <div className="fa-conf-replace">
          <p id={warnId} className="fa-conf-warn" data-testid="replace-warning">
            {warnText}
          </p>
          <p className="fa-conf-scope">
            {fillSlots(t('act.replace.scope', { session: '{session}', persistent: '{persistent}' }), {
              session: <TermTip term="scope_session"><span data-term="scope_session" tabIndex={0} className="fa-termword">session scope</span></TermTip>,
              persistent: <TermTip term="scope_persistent"><span data-term="scope_persistent" tabIndex={0} className="fa-termword">persistent scope</span></TermTip>,
            })}
          </p>
          {strong && choice === 'replace' && (
            <label className="fa-radio"><input type="checkbox" checked={understood} onChange={e => setUnderstood(e.target.checked)} />{t('act.replace.strong')}</label>
          )}
          <div className="fa-conf-guide" data-testid="guide-text">
            <strong>{t('act.replace.guide', { app: guideApp })}</strong>
            <span>{t('act.replace.guide.body', { app: guideApp })}</span>
            {onRetry && <button type="button" onClick={onRetry}>{t('act.replace.retry')}</button>}
          </div>
        </div>
      )}
      {advice.tier === 'guide' && (
        <div className="fa-conf-replace" data-testid="guide-text">
          <span>{t('act.replace.guideOnly', { app: appName || t('act.replace.app.other') })}</span>
          {onRetry && <button type="button" onClick={onRetry}>{t('act.replace.retry')}</button>}
        </div>
      )}
      {advice.tier === 'protected' && (
        <div className="fa-conf-replace" data-testid="protected-note">
          <span>{t('act.replace.protected')}</span>
          {onReveal && advice.protectedPath && <button type="button" onClick={() => onReveal(advice.protectedPath!)}>{t('act.replace.reveal')}</button>}
        </div>
      )}
      <div className="fa-modal-actions">
        <button type="button" onClick={onCancel}>{t('act.conflict.cancel')}</button>
        <button type="button" className="primary" disabled={goDisabled} onClick={() => (choice === 'replace' && onReplace ? onReplace() : onConfirm(choice === 'both'))}>{t('act.conflict.go')}</button>
      </div>
    </Modal>
  );
}

type HeldProps = { collectionCount: number; onCancel(): void; onConfirm(): void };
export function HeldDialog({ collectionCount, onCancel, onConfirm }: HeldProps): React.JSX.Element {
  return (
    <Modal title={tn('act.held.title', { n: collectionCount })} onClose={onCancel}>
      <p>{t('act.held.body')}</p>
      <div className="fa-modal-actions">
        <button type="button" onClick={onCancel}>{t('act.held.cancel')}</button>
        <button type="button" className="primary" onClick={onConfirm}>{t('act.held.go')}</button>
      </div>
    </Modal>
  );
}
