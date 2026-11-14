import { useEffect } from 'react';
import type { Api, ReasonDto } from '../../api';
import { formatNumber, t } from '../../i18n';
import { resolveReasonNames, useReasonNames } from '../../store/reasonNames';
import { Tooltip } from '../ui/Tooltip';

/** Một lý do thành một câu ngắn (vi+en). Chưa có tên thì hiện số id để không trống. */
export function reasonText(r: ReasonDto, names: Record<string, string>): string {
  switch (r.kind) {
    case 'manual': return t('act.why.manual');
    case 'collection': return t('act.why.collection', { name: names[`c:${r.collectionId}`] ?? `#${r.collectionId}` });
    case 'case_app': return t('act.why.caseApp', { app: names[`a:${r.collectionId}`] || t('act.why.anApp'), name: names[`c:${r.collectionId}`] ?? `#${r.collectionId}` });
    case 'auto': return t('act.why.auto', { app: r.app });
  }
}
/** Thứ tự ưu tiên khi chỉ hiện một lý do: người dùng chủ động trước, rồi theo Dự án/Case, rồi theo app. */
const ORDER: Record<ReasonDto['kind'], number> = { manual: 0, collection: 1, case_app: 2, auto: 3 };

/** Dòng nhỏ "Vì sao đang bật" dưới hàng Active: lý do đầu + "+N" (tooltip liệt kê tất cả). Không có lý do → không vẽ gì. */
export function ReasonLine({ api, reasons }: { api: Api; reasons: readonly ReasonDto[] }): React.JSX.Element | null {
  const names = useReasonNames(s => s.names);
  useEffect(() => { reasons.forEach(r => resolveReasonNames(api, r)); }, [api, reasons]);
  if (reasons.length === 0) return null;
  const sorted = [...reasons].sort((a, b) => ORDER[a.kind] - ORDER[b.kind]);
  const more = sorted.length - 1;
  const all = sorted.map(r => reasonText(r, names));
  return (
    <div className="fa-act-why" data-testid="active-why">
      <span>{all[0]}</span>
      {more > 0 && <Tooltip label={<span className="fa-act-why-all">{all.map((x, i) => <span key={i}>{x}</span>)}</span>}><span className="fa-act-why-more" tabIndex={0} data-testid="active-why-more" aria-label={t('act.why.all', { n: formatNumber(sorted.length) })}>+{more}</span></Tooltip>}
    </div>
  );
}
