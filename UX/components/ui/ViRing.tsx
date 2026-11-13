import { t, tn } from '../../i18n';
import { Tooltip } from './Tooltip';
import './viRing.css';

/** Tổng ký tự tiếng Việt chuẩn của app (thống nhất với bộ lọc thư viện); dùng khi backend chưa trả `total`. */
export const VI_TOTAL = 186;
const R = 5.5; const C = 2 * Math.PI * R;

/** Vòng độ phủ tiếng Việt `x/186` (A4). Xanh khi đủ, hổ phách khi > 60 %, đỏ khi thấp hơn; luôn kèm số để không chỉ dựa vào màu. */
export function ViRing({ missing, total, supports }: { missing: number; total: number; supports?: boolean }): React.JSX.Element {
  const all = total > 0 ? total : VI_TOTAL;
  const ok = Math.max(0, Math.min(all, all - missing));
  const p = ok / all;
  const tone = missing === 0 && supports !== false ? 'ok' : p > 0.6 ? 'mid' : 'low';
  const label = missing === 0 && supports !== false ? t('strip.viOk') : t('strip.viRing', { ok, total: all });
  return (
    <Tooltip label={missing === 0 ? t('strip.viOk') : tn('strip.viMissing', { n: missing })}>
      <span className="fa-viring" data-testid="vi-ring" data-tone={tone} role="img" aria-label={label}>
        <svg viewBox="0 0 14 14" width="14" height="14" aria-hidden>
          <circle cx="7" cy="7" r={R} className="trk" />
          <circle cx="7" cy="7" r={R} className="val" strokeDasharray={`${(C * p).toFixed(2)} ${C.toFixed(2)}`} transform="rotate(-90 7 7)" />
        </svg>
        <span className="n">{ok}/{all}</span>
      </span>
    </Tooltip>
  );
}
