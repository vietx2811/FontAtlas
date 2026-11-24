// Compare: hai cột cùng tên hiển thị nhưng khác file (thư viện thật hay có bản trùng) → thêm dòng phụ để phân biệt.
import type { FaceRowDto } from '../../bindings';
import { faceLabel } from '../../store/nameClash';
import { t } from '../../i18n';
import { Tooltip } from '../ui/Tooltip';

type R = Pick<FaceRowDto, 'faceId' | 'hashHex' | 'faceIndex' | 'family' | 'subfamily' | 'path'>;
export type DupeHint = { line: string; path: string | null };
const key = (r: R) => `${r.hashHex}:${r.faceIndex}`;
const parts = (p: string) => p.split(/[\\/]+/).filter(Boolean);

/** Gợi ý phân biệt cho các font đang hiện cùng một lúc: chỉ font có tên trùng với font KHÁC FILE mới có gợi ý (cùng một font ở 2 cột thì không phải trùng). */
export function dupeHints(rows: ReadonlyArray<R | null | undefined>): Map<string, DupeHint> {
  const byLabel = new Map<string, Map<string, R>>();
  for (const r of rows) {
    if (!r) continue;
    const label = faceLabel(r);
    const g = byLabel.get(label) ?? new Map<string, R>();
    g.set(key(r), r); byLabel.set(label, g);
  }
  const out = new Map<string, DupeHint>();
  for (const g of byLabel.values()) {
    if (g.size < 2) continue;
    const list = [...g.values()];
    const base = (r: R) => (r.path ? parts(r.path).slice(-1)[0] ?? '' : '');
    const baseCount = new Map<string, number>();
    for (const r of list) baseCount.set(base(r), (baseCount.get(base(r)) ?? 0) + 1);
    for (const r of list) {
      const b = base(r); const ps = r.path ? parts(r.path) : [];
      // Tên file trùng nhau (khác thư mục) → thêm thư mục chứa; không có đường dẫn → mã ngắn của file.
      const line = !b ? r.hashHex.slice(0, 8) : (baseCount.get(b) ?? 0) > 1 && ps.length > 1 ? `${ps[ps.length - 2]}/${b}` : b;
      out.set(key(r), { line: r.faceIndex > 0 ? `${line} #${r.faceIndex}` : line, path: r.path });
    }
  }
  return out;
}

export const dupeHintOf = (m: Map<string, DupeHint>, r: R): DupeHint | undefined => m.get(key(r));

/** Dòng phụ dưới tên; tooltip là đường dẫn đầy đủ. Không vẽ gì khi không trùng. */
export function DupeLine({ hint }: { hint: DupeHint | undefined }): React.JSX.Element | null {
  if (!hint) return null;
  const el = <small className="fa-dupe-line" data-testid="dupe-line" tabIndex={hint.path ? 0 : undefined} aria-label={hint.path ? `${t('fc.dupe.file')}: ${hint.path}` : undefined}>{hint.line}</small>;
  return hint.path ? <Tooltip label={hint.path}>{el}</Tooltip> : el;
}
