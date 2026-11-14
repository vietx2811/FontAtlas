import { useCallback, useEffect, useState } from 'react';
import type { Api, OrganizedStatusDto, PackInfoDto } from '../../api';
import { formatNumber, t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { formatBytes } from '../duplicates/dupText';
import { pickFolderWithDialog } from '../sidebar/useDropPaths';
import { NoteMark } from '../ui/noteMark';
import { OrganizedImportDialog } from './OrganizedImportDialog';
import { OrganizedSetup } from './OrganizedSetup';
import { ResortDialog } from './ResortDialog';
import { RestorePackDialog } from './RestorePackDialog';
import './organized.css';

type Dlg = 'import' | 'resort' | 'restore' | null;

/** Cài đặt ▸ Thư viện: Organized Library (trạng thái, chép thêm, sắp xếp lại, chụp gói, khôi phục từ gói). */
export function OrganizedPanel({ api, pick = pickFolderWithDialog }: { api: Api; pick?: () => Promise<string | null> }): React.JSX.Element {
  const [st, setSt] = useState<OrganizedStatusDto | null>(null);
  const [pending, setPending] = useState<PackInfoDto | null>(null);
  const [dlg, setDlg] = useState<Dlg>(null);
  const refresh = useCallback(() => { api.organizedStatus().then(setSt, () => undefined); api.packRestorePending().then(setPending, () => undefined); }, [api]);
  useEffect(() => { refresh(); }, [refresh]);
  const snapshot = async () => { try { await api.organizedSnapshotNow(); useNoticeStore.getState().push({ kind: 'info', text: t('org.snap.done') }); refresh(); } catch (e) { useNoticeStore.getState().push({ kind: 'error', text: t('org.err.other', { msg: (e as Error).message }) }); } };
  const close = () => { setDlg(null); refresh(); };
  return (
    <div className="fa-set-group" data-testid="organized-panel">
      <span>{t('org.title')}</span>
      <p className="fa-set-note">{t('org.intro')}</p>
      {pending && <p className="fa-org-warn" data-testid="org-pending">{t('org.pack.pending', { ver: pending.appVersion, files: formatNumber(pending.files) })}</p>}
      {st && !st.configured && <OrganizedSetup api={api} pick={pick} onCreated={s => { setSt(s); useNoticeStore.getState().push({ kind: 'info', text: t('org.created') }); }} />}
      {st?.configured && (
        <>
          <dl className="fa-org-status" data-testid="org-status">
            <dt>{t('org.st.root')}</dt><dd>{st.root}</dd>
            <dt>{t('org.st.files')}</dt><dd>{formatNumber(st.files)} · {formatBytes(st.bytes)}</dd>
            <dt>{t('org.st.free')}</dt><dd>{formatBytes(st.freeBytes)}</dd>
            <dt>{t('org.st.snapshots')}</dt><dd>{st.snapshots > 0 ? t('org.st.snapshotsN', { n: formatNumber(st.snapshots) }) : t('org.st.none')}</dd>
          </dl>
          {st.isCloud && <p className="fa-org-warn"><NoteMark inline text={t('org.cloud.tip')} /> {t('org.cloud', { provider: t('org.cloud.generic') })}</p>}
          {!st.available && <p className="fa-org-err" role="alert" data-testid="org-unavailable">{t('org.st.unavailable')}</p>}
          <div className="fa-org-actions">
            <button type="button" data-testid="org-open-import" disabled={!st.available} onClick={() => setDlg('import')}>{t('org.act.import')}</button>
            <button type="button" data-testid="org-open-resort" disabled={!st.available} onClick={() => setDlg('resort')}>{t('org.act.resort')}</button>
            <button type="button" disabled={!st.available} onClick={() => void snapshot()}>{t('org.act.snapshot')}</button>
          </div>
        </>
      )}
      <div className="fa-org-actions"><button type="button" data-testid="org-open-restore" onClick={() => setDlg('restore')}>{t('org.act.restore')}</button></div>
      {dlg === 'import' && <OrganizedImportDialog api={api} onClose={close} />}
      {dlg === 'resort' && <ResortDialog api={api} onClose={close} />}
      {dlg === 'restore' && <RestorePackDialog api={api} pick={pick} onClose={close} />}
    </div>
  );
}
