import { hasKey, t, type I18nKey } from '../../i18n';
import type { ResolveFailureDto, ResolveReasonDto } from '../../api';
import { baseName } from './dupPlan';
import { localeTag } from './dupLocale';

export function formatBytes(n: number): string {
  const num = new Intl.NumberFormat(localeTag(), { maximumFractionDigits: 1 });
  if (!Number.isFinite(n) || n < 0) return '—';
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${num.format(n / 1024)} KB`;
  if (n < 1024 ** 3) return `${num.format(n / 1024 / 1024)} MB`;
  return `${num.format(n / 1024 ** 3)} GB`;
}

type Vars = Record<string, string | number>;
const pick = (prefix: string, code: string, fallback: I18nKey, vars: Vars = {}): string => {
  const key = `${prefix}.${code}`;
  return hasKey(key) ? t(key as I18nKey, vars) : t(fallback, vars);
};

/** Vì sao giữ bản này. Mã lạ (backend mới) vẫn ra một câu thường, không lộ mã. */
export const reasonText = (r: ResolveReasonDto): string => pick('dup.reason', r.code, 'dup.reason.unknown', r.args);
/** Vì sao máy không tự quyết. */
export const reviewText = (r: ResolveReasonDto): string => pick('dup.review', r.code, 'dup.review.unknown', r.args);
export const warnText = (r: ResolveReasonDto): string => pick('dup.warn', r.code, 'dup.warn.unknown', r.args);
export const blockedText = (code: string | null): string => pick('dup.blocked', code ?? 'unknown', 'dup.blocked.unknown');
/** Lỗi theo `code`, kèm tên tệp. `detail` thô không bao giờ nằm trong câu này. */
export const failText = (f: ResolveFailureDto): string => pick('dup.fail', f.code, 'dup.fail.unknown', { name: f.path ? baseName(f.path) : '—' });
/** Chỉ phần lý do của một lỗi (không có tên tệp), để gộp "N tệp: lý do". */
export const failWhy = (code: string): string => failText({ path: 'x', code, detail: null }).replace(/^[^:]*:\s*/, '');
