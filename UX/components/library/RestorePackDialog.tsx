import { useState } from 'react';
import type { Api, PackInfoDto } from '../../api';
import { CmdError } from '../../api';
import { formatDate, formatNumber, t } from '../../i18n';
import { Modal } from '../Modal';
import { NoteMark } from '../ui/noteMark';
import './organized.css';

/**
 * Khôi phục từ gói: chọn thư mục gói → kiểm toàn vẹn (pack_inspect) → (tuỳ chọn) chỉ lại nơi các nguồn đã chuyển → xếp lịch.
 * Khôi phục chạy ở LẦN KHỞI ĐỘNG SAU (DB hiện tại được dời đi, không xoá): hộp thoại nói rõ và có nút Huỷ lịch.
 */
export function RestorePackDialog({ api, pick, onClose }: { api: Api; pick(): Promise<string | null>; onClose(): void }): React.JSX.Element {
  const [info, setInfo] = useState<PackInfoDto | null>(null);
  const [path, setPath] = useState<string | null>(null);
  const [remap, setRemap] = useState<Record<string, string>>({});
  const [staged, setStaged] = useState(false);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const choose = async () => {
    setErr(null); setBusy(true);
    try { const p = await pick(); if (!p) return; const i = await api.packInspect(p); setPath(p); setInfo(i); setRemap(Object.fromEntries(i.sourceRoots.map(r => [r, r]))); }
    catch (e) { setErr(e instanceof CmdError && e.code === 'invalid_pack' ? t('org.pack.invalid') : t('org.err.other', { msg: (e as Error).message })); }
    finally { setBusy(false); }
  };
  const stage = async () => {
    if (!path || !info) return;
    setBusy(true); setErr(null);
    try { await api.packRestoreStage(path, info.sourceRoots.filter(r => remap[r] && remap[r] !== r).map(r => ({ from: r, to: remap[r] }))); setStaged(true); }
    catch (e) { setErr(t('org.err.other', { msg: (e as Error).message })); }
    finally { setBusy(false); }
  };
  const unstage = async () => { setBusy(true); try { await api.packRestoreCancel(); setStaged(false); } finally { setBusy(false); } };
  return (
    <Modal title={t('org.pack.title')} onClose={onClose} className="fa-org-dialog">
      {!info && <><p>{t('org.pack.intro')}</p><button type="button" disabled={busy} data-testid="org-pack-choose" onClick={() => void choose()}>{t('org.pack.choose')}</button></>}
      {info && !staged && (
        <>
          <p data-testid="org-pack-info">{t('org.pack.found', { date: formatDate(info.createdAt * 1000, { day: '2-digit', month: '2-digit', year: 'numeric' }), ver: info.appVersion, files: formatNumber(info.files), faces: formatNumber(info.faces) })}</p>
          <p className="fa-org-note">{t('org.pack.noModel')}</p>
          {info.sourceRoots.length > 0 && (
            <div data-testid="org-pack-remap">
              <p className="fa-org-note">{t('org.pack.remap')} <NoteMark inline kind="info" text={t('org.pack.remap.tip')} /></p>
              {info.sourceRoots.map(r => <label key={r} className="fa-org-remap"><span className="path">{r}</span>
                <input value={remap[r] ?? r} aria-label={t('org.pack.remapAria', { path: r })} onChange={e => setRemap(m => ({ ...m, [r]: e.target.value }))} /></label>)}
            </div>
          )}
          <p className="fa-org-warn">{t('org.pack.when')}</p>
        </>
      )}
      {staged && <p role="status" data-testid="org-pack-staged">{t('org.pack.staged')}</p>}
      {err && <p className="fa-org-err" role="alert">{err}</p>}
      <div className="fa-modal-actions">
        {staged ? <button type="button" disabled={busy} onClick={() => void unstage()}>{t('org.pack.unstage')}</button> : <button type="button" onClick={onClose}>{t('org.cancel')}</button>}
        {info && !staged && <button type="button" className="primary" data-testid="org-pack-stage" disabled={busy} onClick={() => void stage()}>{t('org.pack.stage')}</button>}
        {staged && <button type="button" className="primary" onClick={onClose}>{t('org.close')}</button>}
      </div>
    </Modal>
  );
}
