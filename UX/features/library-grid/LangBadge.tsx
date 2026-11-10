import type { LangBadgeDto } from '../../bindings';
import { t } from '../../i18n';
import { Tooltip } from '../../components/ui/Tooltip';
import { langName } from '../../components/filter/langNames';

const R = 5; const C = 2 * Math.PI * R;

/** Tổng số ký tự base tiếng Việt khi cần dựng chỉ báo từ `viMissing` (font chưa có hồ sơ ngôn ngữ). */
export const VI_BASE_TOTAL = 134;
/** Dựng từ số ký tự thiếu: thiếu dù chỉ 1 ký tự tiếng Việt vẫn là cảnh báo (không có mức "gần đủ" khi chưa có hồ sơ Hyperglot). */
export const viBadgeFromMissing = (viMissing: number): LangBadgeDto => {
  const have = Math.max(0, VI_BASE_TOTAL - viMissing);
  const ratio = have / VI_BASE_TOTAL;
  return { code: 'vie', bcp47: 'vi', state: viMissing === 0 ? 'full' : ratio >= 0.25 ? 'partial' : 'low', have, total: VI_BASE_TOTAL };
};

/**
 * Chỉ báo ngôn ngữ trên thẻ font (thay `−N`, không có số): mã ngôn ngữ viết hoa + vòng độ phủ.
 * Đủ = vòng đầy · Gần đủ ≥ 90% = cung gần kín · Thiếu 25–90% = cung theo tỷ lệ (cảnh báo) · < 25% = vòng rỗng gạch ngang.
 * Bốn trạng thái khác HÌNH, không chỉ khác màu. Đã tính nhưng dưới "gần đủ" mà không có số đếm (`below`) = vòng rỗng gạch ngang, tooltip không in x/y. Chưa phân tích thì không hiện. Chi tiết (thiếu bao nhiêu) nằm ở tooltip.
 */
export function LangBadge({ badge }: { badge: LangBadgeDto }): React.JSX.Element | null {
  if (badge.state === 'unknown') return null;
  const lang = langName({ code: badge.code, bcp47: badge.bcp47 });
  const miss = Math.max(0, badge.total - badge.have);
  const warn = badge.state === 'partial' || badge.state === 'low' || badge.state === 'below';
  const ratio = badge.total ? badge.have / badge.total : 0;
  const tip = t(`badge.${badge.state}` as 'badge.full', { lang, have: badge.have, total: badge.total, miss });
  return (
    <Tooltip label={tip}>
      <span className={`fa-vi fa-lang-badge ${warn ? 'miss' : badge.state === 'near' ? 'near' : 'ok'}`} data-testid="vi-mark" data-lang={badge.code} data-state={badge.state} data-warn={warn ? 'true' : undefined} role="img" aria-label={tip}>
        <small className="fa-vi-code">{badge.bcp47.toUpperCase()}</small>
        <svg viewBox="0 0 14 14" width="14" height="14" aria-hidden className="fa-vi-ring">
          {badge.state === 'full'
            ? <circle cx="7" cy="7" r="6" className="fill" />
            : <>
              <circle cx="7" cy="7" r={R} className="track" />
              {badge.state === 'low' || badge.state === 'below'
                ? <path d="M3 11L11 3" className="strike" />
                : <circle cx="7" cy="7" r={R} className="arc" strokeDasharray={`${C * ratio} ${C}`} transform="rotate(-90 7 7)" />}
            </>}
        </svg>
      </span>
    </Tooltip>
  );
}
