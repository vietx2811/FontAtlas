import { useEffect } from 'react';
import type { Api, ReasonDto, ReasonNamesDto } from '../../api';
import { formatNumber, t } from '../../i18n';
import { resolveReasonNames, useReasonNames } from '../../store/reasonNames';
import { Tooltip } from '../ui/Tooltip';
import './activeRow.css';
import { Icon, type IconName } from '../sidebar/icons';

/** Một lý do thành một câu ngắn (vi+en). Tên ưu tiên `dto` (backend: đúng không gian id, kể cả Case trong thùng rác), rồi tên tra lẻ; chưa có thì hiện số id. */
export function reasonText(r: ReasonDto, names: Record<string, string>, dto?: ReasonNamesDto): string {
  switch (r.kind) {
    case 'manual': return t('act.why.manual');
    case 'case': return t('act.why.case', { name: dto?.collectionName ?? names[`k:${r.caseId}`] ?? `#${r.caseId}` });
    case 'collection': return t('act.why.collection', { name: dto?.collectionName ?? names[`c:${r.collectionId}`] ?? `#${r.collectionId}` });
    case 'case_app': return t('act.why.caseApp', { app: (dto?.appNames.length ? dto.appNames.join(', ') : undefined) || names[`a:${r.caseId}`] || t('act.why.anApp'), name: dto?.collectionName ?? names[`k:${r.caseId}`] ?? `#${r.caseId}` });
    case 'auto': return t('act.why.auto', { app: r.app });
  }
}
/** Thứ tự ưu tiên khi chỉ hiện một lý do: người dùng chủ động trước, rồi Case, rồi Dự án, rồi theo app. */
const ORDER: Record<ReasonDto['kind'], number> = { manual: 0, case: 1, collection: 2, case_app: 3, auto: 4 };

/** Dòng nhỏ "Vì sao đang bật" dưới hàng Active: lý do đầu + "+N" (tooltip liệt kê tất cả). Không có lý do → không vẽ gì.
 *  `reasonNames` (cùng thứ tự `reasons`, từ activation_state) có thì dùng thẳng, không gọi lẻ. */
export function ReasonLine({ api, reasons, reasonNames }: { api: Api; reasons: readonly ReasonDto[]; reasonNames?: readonly ReasonNamesDto[] }): React.JSX.Element | null {
  const names = useReasonNames(s => s.names);
  const hasDto = reasonNames !== undefined && reasonNames.length === reasons.length;
  useEffect(() => { if (!hasDto) reasons.forEach(r => resolveReasonNames(api, r)); }, [api, reasons, hasDto]);
  if (reasons.length === 0) return null;
  const sorted = reasons.map((r, i) => ({ r, dto: hasDto ? reasonNames[i] : undefined })).sort((a, b) => ORDER[a.r.kind] - ORDER[b.r.kind]);
  const more = sorted.length - 1;
  const all = sorted.map(x => reasonText(x.r, names, x.dto));
  return (
    <div className="fa-act-why" data-testid="active-why">
      <span>{all[0]}</span>
      {more > 0 && <Tooltip label={<span className="fa-act-why-all">{all.map((x, i) => <span key={i}>{x}</span>)}</span>}><span className="fa-act-why-more" tabIndex={0} data-testid="active-why-more" aria-label={t('act.why.all', { n: formatNumber(sorted.length) })}>+{more}</span></Tooltip>}
    </div>
  );
}

const ICON: Record<ReasonDto['kind'], IconName> = { manual: 'power', case: 'pair', collection: 'folder', case_app: 'panel', auto: 'clock' };
/** Tên ngắn của một lý do (icon + vài chữ) cho thẻ/dòng font; câu đầy đủ nằm ở tooltip (`reasonText`). */
export function reasonShort(r: ReasonDto, names: Record<string, string>, dto?: ReasonNamesDto): { icon: IconName; label: string } {
  const icon = ICON[r.kind];
  switch (r.kind) {
    case 'manual': return { icon, label: t('act.why.s.manual') };
    case 'case': return { icon, label: t('act.why.s.case', { name: dto?.collectionName ?? names[`k:${r.caseId}`] ?? `#${r.caseId}` }) };
    case 'collection': return { icon, label: t('act.why.s.collection', { name: dto?.collectionName ?? names[`c:${r.collectionId}`] ?? `#${r.collectionId}` }) };
    case 'case_app': return { icon, label: t('act.why.s.app', { app: (dto?.appNames.length ? dto.appNames.join(', ') : undefined) || names[`a:${r.caseId}`] || t('act.why.anApp') }) };
    case 'auto': return { icon, label: t('act.why.s.auto') };
  }
}

/** Lý do chính trên thẻ font ở "Đang bật": icon + tên ngắn, "+N" nếu còn lý do khác; tất cả trong tooltip. Không có lý do → không vẽ. */
export function ReasonBadge({ reasons, reasonNames }: { reasons: readonly ReasonDto[] | undefined; reasonNames?: readonly ReasonNamesDto[] }): React.JSX.Element | null {
  const names = useReasonNames(s => s.names);
  if (!reasons || reasons.length === 0) return null;
  const hasDto = reasonNames !== undefined && reasonNames.length === reasons.length;
  const sorted = reasons.map((r, i) => ({ r, dto: hasDto ? reasonNames[i] : undefined })).sort((a, b) => ORDER[a.r.kind] - ORDER[b.r.kind]);
  const main = reasonShort(sorted[0].r, names, sorted[0].dto);
  const more = sorted.length - 1;
  const all = sorted.map(x => reasonText(x.r, names, x.dto));
  return (
    <Tooltip label={<span className="fa-act-why-all">{all.map((x, i) => <span key={i}>{x}</span>)}</span>}>
      <span className="fa-reason" data-testid="cell-reason" data-kind={sorted[0].r.kind} tabIndex={0} aria-label={more > 0 ? `${all[0]} · ${t('act.why.all', { n: formatNumber(sorted.length) })}` : all[0]} onClick={e => e.stopPropagation()} onKeyDown={e => e.stopPropagation()}>
        <Icon name={main.icon} size={12} /><span className="fa-reason-t">{main.label}</span>{more > 0 && <span className="fa-reason-n" data-testid="cell-reason-more">+{more}</span>}
      </span>
    </Tooltip>
  );
}
