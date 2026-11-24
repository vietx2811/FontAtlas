import { useState } from 'react';
import type { Api, OrganizedLocationCheckDto, OrganizedStatusDto } from '../../api';
import { CmdError } from '../../api';
import { t } from '../../i18n';
import { formatBytes } from '../duplicates/dupText';
import { NoteMark } from '../ui/noteMark';
import './organized.css';

/** Lý do chọn thư mục không dùng được (mã của organized_check_location / organized_create) → câu dễ hiểu. */
export function locationProblemText(code: string | null): string | null {
  if (!code) return null;
  if (code === 'not_a_directory') return t('org.err.notDir');
  if (code === 'not_writable') return t('org.err.notWritable');
  if (code === 'nested_source') return t('org.err.nested');
  if (code === 'already_configured') return t('org.err.already');
  return t('org.err.other', { msg: code });
}

/**
 * Chọn vị trí Organized Library: kiểm (không để lại gì trên đĩa), cảnh báo ổ cloud (không chặn), rồi tạo.
 * Dùng chung cho Onboarding và Cài đặt ▸ Thư viện. `pick` = hộp thoại chọn thư mục (có thể thay trong test/QA).
 */
export function OrganizedSetup({ api, pick, onCreated, disabled }: { api: Api; pick(): Promise<string | null>; onCreated(s: OrganizedStatusDto): void; disabled?: boolean }): React.JSX.Element {
  const [check, setCheck] = useState<OrganizedLocationCheckDto | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const choose = async () => {
    setErr(null); setBusy(true);
    try { const p = await pick(); if (p) setCheck(await api.organizedCheckLocation(p)); }
    catch (e) { setErr(e instanceof CmdError ? locationProblemText(e.code) ?? e.message : (e as Error).message); }
    finally { setBusy(false); }
  };
  const create = async () => {
    if (!check || check.problem) return;
    setErr(null); setBusy(true);
    try { onCreated(await api.organizedCreate(check.path)); }
    catch (e) { setErr(e instanceof CmdError ? locationProblemText(e.code) ?? e.message : (e as Error).message); }
    finally { setBusy(false); }
  };
  const problem = locationProblemText(check?.problem ?? null);
  return (
    <div className="fa-org-setup" data-testid="org-setup">
      <button type="button" disabled={busy || disabled} onClick={() => void choose()}>{check ? t('org.choose.again') : t('org.choose')}</button>
      {check && (
        <div className="fa-org-check" data-testid="org-check">
          <span className="path">{check.path}</span>
          <span className="meta">{t('org.free', { size: formatBytes(check.freeBytes) })}</span>
          {check.isCloud && <p className="fa-org-warn" data-testid="org-cloud-warn"><NoteMark inline text={t('org.cloud.tip')} /> {t('org.cloud', { provider: check.cloudProvider ?? t('org.cloud.generic') })}</p>}
          {problem && <p className="fa-org-err" role="alert">{problem}</p>}
          <button type="button" className="primary" data-testid="org-create" disabled={busy || !!check.problem || disabled} onClick={() => void create()}>{t('org.create')}</button>
        </div>
      )}
      {err && <p className="fa-org-err" role="alert">{err}</p>}
    </div>
  );
}
