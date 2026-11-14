import { useEffect, useRef, useState } from 'react';
import type { Api, OrganizePlanDto, OrganizeProgressDto, OrganizeResultDto } from '../../api';
import { CmdError } from '../../api';
import { formatNumber, t } from '../../i18n';
import { Modal } from '../Modal';
import { formatBytes } from '../duplicates/dupText';
import { NoteMark } from '../ui/noteMark';
import './organized.css';

const reasonKey = (code: string) => (['cloud_not_downloaded', 'source_missing', 'verify_failed', 'disk_full', 'io'].includes(code) ? `org.issue.${code}` : 'org.issue.io') as 'org.issue.io';

/** Tóm tắt lỗi từng file theo mã: "2 chưa tải về máy; 1 không đọc được". */
export function issueSummary(issues: ReadonlyArray<{ code: string }>): string {
  const by = new Map<string, number>();
  for (const i of issues) by.set(i.code, (by.get(i.code) ?? 0) + 1);
  return [...by.entries()].map(([c, n]) => `${formatNumber(n)} ${t(reasonKey(c))}`).join('; ');
}

type Phase = 'plan' | 'running' | 'done';

/**
 * Chép font vào Organized Library: xem trước (không ghi gì) → xác nhận → chép có tiến độ + Huỷ → kết quả.
 * Chữ xác nhận nói thẳng: CHÉP chứ không di chuyển, bản gốc nguyên vẹn, giữ tên file, tốn thêm dung lượng. Không chặn theo giấy phép.
 */
export function OrganizedImportDialog({ api, onClose, onFinished }: { api: Api; onClose(): void; onFinished?(r: OrganizeResultDto): void }): React.JSX.Element {
  const [phase, setPhase] = useState<Phase>('plan');
  const [plan, setPlan] = useState<OrganizePlanDto | null>(null);
  const [prog, setProg] = useState<OrganizeProgressDto | null>(null);
  const [result, setResult] = useState<OrganizeResultDto | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const alive = useRef(true);
  useEffect(() => { alive.current = true; api.organizedPlan({ sourceIds: null }).then(p => { if (alive.current) setPlan(p); }, e => { if (alive.current) setErr(msgOf(e)); }); return () => { alive.current = false; }; }, [api]);
  const start = async () => {
    setPhase('running'); setErr(null);
    try { const r = await api.organizedImport({ sourceIds: null }, p => { if (alive.current) setProg(p); }); if (!alive.current) return; setResult(r); setPhase('done'); onFinished?.(r); }
    catch (e) { if (alive.current) { setErr(msgOf(e)); setPhase('plan'); } }
  };
  const close = () => { if (phase === 'running') return; onClose(); };
  return (
    <Modal title={t('org.imp.title')} onClose={close} className="fa-org-dialog">
      {phase === 'plan' && (
        <>
          {!plan && !err && <p role="status">{t('org.imp.loading')}</p>}
          {plan && (
            <>
              <p data-testid="org-copy-note">{t('org.imp.copyNote', { size: formatBytes(plan.bytes) })}</p>
              <dl className="fa-org-plan" data-testid="org-plan">
                <dt>{t('org.imp.willCopy')}</dt><dd>{formatNumber(plan.toCopy)} · {formatBytes(plan.bytes)}</dd>
                <dt>{t('org.imp.alreadyThere')}</dt><dd>{formatNumber(plan.alreadyThere)}</dd>
                {plan.cloudNotDownloaded > 0 && <><dt><NoteMark inline text={t('org.imp.cloud.tip')} /> {t('org.imp.cloud')}</dt><dd>{formatNumber(plan.cloudNotDownloaded)}</dd></>}
                <dt>{t('org.imp.freeSpace')}</dt><dd>{formatBytes(plan.freeBytes)} {plan.enoughSpace ? '✓' : ''}</dd>
              </dl>
              {!plan.enoughSpace && <p className="fa-org-err" role="alert">{t('org.imp.noSpace')}</p>}
              <p className="fa-org-note">{t('org.imp.license')}</p>
            </>
          )}
          {err && <p className="fa-org-err" role="alert">{err}</p>}
          <div className="fa-modal-actions">
            <button type="button" onClick={onClose}>{t('org.cancel')}</button>
            <button type="button" className="primary" data-testid="org-start" disabled={!plan || !plan.enoughSpace || plan.toCopy === 0} onClick={() => void start()}>{t('org.imp.start')}</button>
          </div>
        </>
      )}
      {phase === 'running' && (
        <>
          <progress data-testid="org-progress" max={prog?.total || 1} value={prog?.done ?? 0} aria-label={t('org.imp.running')} />
          <p role="status">{t('org.imp.progress', { done: formatNumber(prog?.done ?? 0), total: formatNumber(prog?.total ?? 0), size: formatBytes(prog?.bytesDone ?? 0) })}{prog && prog.failed > 0 ? ` · ${t('org.imp.failedN', { n: formatNumber(prog.failed) })}` : ''}</p>
          <div className="fa-modal-actions"><button type="button" data-testid="org-cancel-run" onClick={() => void api.organizedCancel()}>{t('org.cancel')}</button></div>
        </>
      )}
      {phase === 'done' && result && (
        <>
          <p role="status" data-testid="org-done">{result.cancelled ? t('org.imp.cancelled', { n: formatNumber(result.copied) }) : t('org.imp.done', { n: formatNumber(result.copied), size: formatBytes(result.bytes) })}</p>
          {result.alreadyThere > 0 && <p className="fa-org-note">{t('org.imp.thereN', { n: formatNumber(result.alreadyThere) })}</p>}
          {result.issueCount > 0 && <p className="fa-org-warn" data-testid="org-issues">{t('org.imp.issues', { list: issueSummary(result.issues) })}</p>}
          {result.snapshot && <p className="fa-org-note">{t('org.imp.snapshot')}</p>}
          <div className="fa-modal-actions"><button type="button" className="primary" onClick={onClose}>{t('org.close')}</button></div>
        </>
      )}
    </Modal>
  );
}
function msgOf(e: unknown): string {
  if (e instanceof CmdError) {
    if (e.code === 'busy') return t('org.err.busy');
    if (e.code === 'not_configured') return t('org.err.notConfigured');
    if (e.code === 'source_unavailable') return t('org.err.unavailable');
    if (e.code === 'disk_full') return t('org.imp.noSpace');
  }
  return t('org.err.other', { msg: (e as Error)?.message ?? String(e) });
}
