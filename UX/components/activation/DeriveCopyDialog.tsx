import { useState } from 'react';
import type { Api } from '../../api';
import { t } from '../../i18n';
import { refreshActivation } from '../../store/activationSync';
import { useDeriveStore } from '../../store/deriveCopy';
import { useNoticeStore } from '../../store/notices';
import { Modal } from '../Modal';
import { NoteMark } from '../ui/noteMark';
import { TERM_NAMES, TermTip } from '../ui/terms';
import { copyDoneText, deriveErrorText, makeRenamedCopy } from './renamedCopy';

/** "Tạo bản sao đổi tên…" cho font VF trùng tên PostScript với bản static. Không bao giờ tạo ngầm; file gốc không đụng; giấy phép chỉ ghi chú. */
export function DeriveCopyDialog({ api }: { api: Api }): React.JSX.Element | null {
  const target = useDeriveStore(s => s.target); const close = useDeriveStore(s => s.close);
  const [busy, setBusy] = useState(false);
  if (!target) return null;
  const go = async () => {
    setBusy(true);
    try {
      const { copy, name } = await makeRenamedCopy(api, target.face);
      close();
      useNoticeStore.getState().push({ kind: 'info', text: copyDoneText(copy, name) });
      void refreshActivation(api);
    } catch (e) { close(); useNoticeStore.getState().push({ kind: 'error', text: deriveErrorText(e) }); } finally { setBusy(false); }
  };
  return (
    <Modal title={t('copy.title', { name: target.label })} onClose={close} className="fa-conf">
      <p>{t('copy.why')}</p>
      <p>{t('copy.what')} <TermTip term="postscript_name"><span className="fa-termword" data-term="postscript_name" tabIndex={0}>{TERM_NAMES.postscript_name}</span></TermTip></p>
      <p className="fa-org-note"><NoteMark inline kind="info" text={t('copy.where.tip')} /> {t('copy.where')}</p>
      <div className="fa-modal-actions">
        <button type="button" onClick={close}>{t('org.cancel')}</button>
        <button type="button" className="primary" data-testid="copy-make" disabled={busy} onClick={() => void go()}>{t('copy.make')}</button>
      </div>
    </Modal>
  );
}
