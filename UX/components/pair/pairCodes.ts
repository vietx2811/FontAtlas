import { hasKey, t, type I18nKey } from '../../i18n';
import { AXIS_META } from '../detail/tabs/measurements';

/** Mã lạ từ backend không bao giờ hiện thô: bỏ gạch dưới, viết hoa chữ đầu. */
export const humanize = (code: string): string => { const x = code.replace(/_/g, ' ').trim(); return x.charAt(0).toUpperCase() + x.slice(1); };
const has = (key: string): key is I18nKey => hasKey(key);
const look = (key: string): string | null => (has(key) ? t(key) : null);

export function warningLabel(code: string): { text: string; gloss: string | null } {
  return { text: look(`pair.warn.${code}`) ?? humanize(code), gloss: look(`pair.warn.${code}.gloss`) };
}
export const gateLabel = (gate: string): string => look(`pair.gate.${gate}`) ?? humanize(gate);

/** `axis:<trục>` | `global` | `skeleton` (xem `candidate_dto` ở backend). */
export function clashLabel(code: string): string {
  if (code.startsWith('axis:')) { const a = code.slice(5); return AXIS_META[a]?.label ?? humanize(a); }
  return look(`pair.clash.${code}`) ?? humanize(code);
}

/** `missing_vietnamese:<a|b>:<n>`: a = tiêu đề, b = nội dung. */
export function blockLabel(code: string): string {
  const m = /^missing_vietnamese:([ab]):(\d+)$/.exec(code);
  if (m) return t(m[1] === 'a' ? 'pair.block.vi.heading' : 'pair.block.vi.body', { n: m[2] });
  return humanize(code.replace(/:/g, ' '));
}
