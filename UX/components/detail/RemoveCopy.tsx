import { useState } from 'react';
import type { Api, FaceKeyDto } from '../../api';
import { CmdError } from '../../api';
import { t } from '../../i18n';
import { refreshActivation } from '../../store/activationSync';
import { useNoticeStore } from '../../store/notices';
import { useUiStore } from '../../store/ui';
import { Modal } from '../Modal';

const say = (kind: 'info' | 'error', text: string, action?: { label: string; run(): void }): void => void useNoticeStore.getState().push({ kind, text, action });

/** Lỗi gỡ/khôi phục bản sao → câu "chuyện gì + làm gì". */
export function removeCopyError(e: unknown): string {
  const code = e instanceof CmdError ? e.code : '';
  const key = ({ not_derived: 'rcopy.err.notDerived', not_allowed: 'rcopy.err.notAllowed', deactivate_failed: 'rcopy.err.deactivate', trash_failed: 'rcopy.err.trash', already_undone: 'rcopy.err.undone', not_found: 'rcopy.err.gone', not_in_library: 'rcopy.err.gone' } as Record<string, string>)[code];
  return key ? t(key as 'rcopy.err.trash') : t('rcopy.err.other', { msg: (e as Error)?.message ?? String(e) });
}

/** Gỡ một bản sao đổi tên: chuyển vào Thùng rác (không xoá vĩnh viễn), tắt trước nếu đang bật; có Hoàn tác (lấy lại từ Thùng rác hoặc tạo lại từ gốc, không tự bật lại). */
export async function removeCopy(api: Api, face: FaceKeyDto, name: string): Promise<boolean> {
  try {
    const r = await api.removeDerivedCopy(face);
    useUiStore.getState().select(null); // face vừa biến khỏi thư viện
    useUiStore.getState().bumpLibrary(true);
    void refreshActivation(api);
    say('info', t(r.wasActive ? 'rcopy.doneActive' : 'rcopy.done', { name }), {
      label: t('dnd.undo'),
      run: () => {
        api.restoreDerivedCopy(r.removalId).then(
          u => { useUiStore.getState().bumpLibrary(true); say('info', t(u.via === 'trash' ? 'rcopy.undone.trash' : 'rcopy.undone.rederived', { name }) + (r.wasActive ? ` ${t('rcopy.undone.off')}` : '')); },
          e => say('error', removeCopyError(e)),
        );
      },
    });
    return true;
  } catch (e) { say('error', removeCopyError(e)); return false; }
}

/** Nút "Gỡ bản sao này" + hộp xác nhận (Detail của một bản sao). */
export function RemoveCopyButton({ api, face, name }: { api: Api; face: FaceKeyDto; name: string }): React.JSX.Element {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const go = async () => { setBusy(true); await removeCopy(api, face, name); setBusy(false); setOpen(false); };
  return (
    <>
      <button type="button" className="fa-p-derived-rm" data-testid="remove-copy" onClick={() => setOpen(true)}>{t('rcopy.button')}</button>
      {open && (
        <Modal title={t('rcopy.title', { name })} onClose={() => setOpen(false)}>
          <p data-testid="remove-copy-body">{t('rcopy.body')}</p>
          <div className="fa-modal-actions">
            <button type="button" onClick={() => setOpen(false)}>{t('org.cancel')}</button>
            <button type="button" className="primary danger" data-testid="remove-copy-go" disabled={busy} onClick={() => void go()}>{t('rcopy.go')}</button>
          </div>
        </Modal>
      )}
    </>
  );
}
