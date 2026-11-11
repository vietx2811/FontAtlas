import type { ResolveResultDto } from '../../api';
import { t } from '../../i18n';
import { Modal } from '../Modal';
import { PARTIAL_CODES } from './dupPlan';
import { failText, } from './dupText';
import { groupsDone } from './applyPlan';

/** Báo thành công một phần: nhóm đã xử lý, nhóm chưa làm được (kèm lý do thường) và việc dở cần chú ý. `detail` thô nằm sau "Chi tiết kỹ thuật". */
export function ResultDialog({ result, onClose, onUndo }: { result: ResolveResultDto; onClose(): void; onUndo?(): void }): React.JSX.Element {
  const notes = result.failed.filter(f => PARTIAL_CODES.has(f.code));
  const fails = result.failed.filter(f => !PARTIAL_CODES.has(f.code));
  const done = groupsDone(result);
  const headline = done === 0 ? t('dup.res.allFailed', { failed: fails.length }) : t('dup.res.partial', { done, failed: fails.length });
  return (
    <Modal title={t('dup.res.title')} onClose={onClose} className="dup-pv">
      <p data-testid="res-headline"><b>{notes.length > 0 && fails.length === 0 ? t('dup.res.attention', { n: notes.length }) : headline}</b></p>
      {fails.length > 0 && <section><h3>{t('dup.res.fails')}</h3>
        <ul className="dup-res" data-testid="res-fails">{fails.map((f, i) => <li key={`${f.path}:${i}`}>{failText(f)}{f.detail && <details><summary>{t('dup.details')}</summary><code>{t('dup.res.detail', { detail: f.detail })}</code></details>}</li>)}</ul></section>}
      {notes.length > 0 && <section><h3>{t('dup.res.notes')}</h3>
        <ul className="dup-res" data-testid="res-notes">{notes.map((f, i) => <li key={`${f.path}:${i}`}>{failText(f)}{f.detail && <details><summary>{t('dup.details')}</summary><code>{t('dup.res.detail', { detail: f.detail })}</code></details>}</li>)}</ul></section>}
      <div className="fa-modal-actions">
        {onUndo && result.done.length > 0 && <button type="button" onClick={onUndo}>{t('dup.res.undo')}</button>}
        <button type="button" className="primary" onClick={onClose}>{t('dup.res.close')}</button>
      </div>
    </Modal>
  );
}
