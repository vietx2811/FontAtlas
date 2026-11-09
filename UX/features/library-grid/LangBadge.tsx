import type { LangBadgeDto } from '../../bindings';
import { t } from '../../i18n';
import { Tooltip } from '../../components/ui/Tooltip';
import { langName } from '../../components/filter/langNames';

/** Chỉ báo ngôn ngữ trên thẻ font, thay `−N`: ● đủ · ◐ gần đủ · −N thiếu (cảnh báo) · ? chưa tính. Ngôn ngữ khác tiếng Việt kèm mã. */
export function LangBadge({ badge }: { badge: LangBadgeDto }): React.JSX.Element {
  const lang = langName({ code: badge.code, bcp47: badge.bcp47 });
  const miss = Math.max(0, badge.total - badge.have);
  const warn = badge.state === 'partial' || badge.state === 'low';
  const mark = badge.state === 'full' ? '●' : badge.state === 'near' ? '◐' : warn ? `−${miss}` : '?';
  const tip = t(`badge.${badge.state}` as 'badge.full', { lang, have: badge.have, total: badge.total, miss });
  return (
    <Tooltip label={tip}>
      <span className={`fa-vi ${warn ? 'miss' : badge.state === 'unknown' ? 'unk' : 'ok'}`} data-testid="vi-mark" data-lang={badge.code} data-state={badge.state} data-warn={warn ? 'true' : undefined}>
        {badge.code !== 'vie' && <small className="fa-vi-code">{badge.code}</small>}{mark}
      </span>
    </Tooltip>
  );
}
