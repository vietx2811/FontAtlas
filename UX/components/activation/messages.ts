import type { AutoDeactivatedDto, FileOutcomeDto, ReconcileReportDto } from '../../api';
import { t, tn } from '../../i18n';

export type Msg = { kind: 'info' | 'error'; text: string };

/** Thông báo cho một kết quả kích hoạt; thành công thì không nói gì (null). */
export function describeOutcome(o: FileOutcomeDto): Msg | null {
  switch (o.kind) {
    case 'failed':
      if (o.code === 'file_changed') return { kind: 'error', text: t('act.msg.changed') };
      if (o.code === 'scope_unsettled') return { kind: 'error', text: t('act.msg.scope_unsettled') };
      if (o.code === 'replace_protected') return { kind: 'error', text: t('act.replace.protected') };
      if (o.code === 'replace_guide_only') return { kind: 'error', text: t('act.msg.replaceGuide') };
      if (o.code === 'replace_conflicts_changed') return { kind: 'error', text: t('act.msg.replaceChanged') };
      if (o.code === 'replace_rollback_failed') return { kind: 'error', text: t('act.msg.replaceRollbackFailed') };
      if (o.code === 'replace_not_effective') return { kind: 'error', text: t('act.msg.replaceNotEffective') };
      if (o.code === 'replace_own_held') return { kind: 'error', text: t('act.msg.replaceOwnHeld') };
      if (o.code === 'replace_own_not_found') return { kind: 'error', text: t('act.msg.replaceOwnNotFound') };
      if (o.code === 'replace_not_registered') return { kind: 'error', text: t('act.msg.replaceNotRegistered') };
      if (o.code === 'cloud_timeout') return { kind: 'error', text: t('act.msg.cloudTimeout') };
      if (o.code === 'cloud_download_failed') return { kind: 'error', text: t('act.msg.cloudFailed', { message: o.message }) };
      if (o.code === 'cloud_stub') return { kind: 'error', text: t('act.msg.cloudStub') };
      return { kind: 'error', text: t('act.msg.failed', { code: o.code, message: o.message }) };
    case 'missing_file': return { kind: 'error', text: t('act.msg.missing') };
    case 'not_downloaded': return { kind: 'error', text: t('act.msg.icloud') };
    case 'busy': return { kind: 'info', text: t('act.msg.busy') };
    default: return null;
  }
}

const CAUSES = ['collection_deleted', 'left_collection', 'no_reason'] as const;
const group = (items: AutoDeactivatedDto[]) => {
  const m = new Map<string, number>();
  for (const i of items) m.set(i.cause, (m.get(i.cause) ?? 0) + 1);
  return m;
};

/** Thông báo sau `reconcile` (lúc mở app hoặc khi dự án đổi). Không có gì xảy ra thì trả mảng rỗng. */
export function reconcileNotices(r: ReconcileReportDto): Msg[] {
  const out: Msg[] = [];
  for (const [cause, n] of group(r.autoDeactivated)) {
    const known = (CAUSES as readonly string[]).includes(cause);
    out.push({ kind: 'info', text: known ? tn('act.rec.auto', { n, cause: t(`act.rec.${cause as (typeof CAUSES)[number]}`) }) : tn('act.rec.autoUnknown', { n, cause }) });
  }
  if (r.autoDeactivateFailed.length) out.push({ kind: 'error', text: tn('act.rec.autoFailed', { n: r.autoDeactivateFailed.length }) });
  const parts = [
    r.reregistered > 0 && tn('act.rec.rereg', { n: r.reregistered }),
    r.missing > 0 && tn('act.rec.missing', { n: r.missing }),
    r.notDownloaded > 0 && tn('act.rec.icloud', { n: r.notDownloaded }),
  ].filter(Boolean) as string[];
  if (parts.length) out.push({ kind: 'info', text: parts.join(' ') });
  return out;
}
