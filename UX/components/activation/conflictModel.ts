import type { ConflictDto, FaceKeyDto, FaceRow } from '../../api';
import { t } from '../../i18n';

/** Một font cần xác nhận: tên family/kiểu đọc từ thư viện, và các bản trùng đang cài ở máy. */
export type ConflictItem = { family: string; style: string; conflicts: ConflictDto[] };
/** Gộp theo family: mỗi family một dòng ("BT Annam Neue · 8 kiểu"). */
export type ConflictGroup = { family: string; styles: string[]; conflicts: ConflictDto[] };

/** Chỉ font mà MỌI bản trùng đều là chính file này (sameFile) mới coi là "đã có sẵn": không hỏi. */
export const isSameFile = (conflicts: readonly ConflictDto[]): boolean => conflicts.length > 0 && conflicts.every(c => c.sameFile);

/** Ai cài? Thư mục con trực tiếp của ~/Library/Fonts/<App>/ → "do <App> cài"; còn lại theo owner. */
export function ownerNote(c: Pick<ConflictDto, 'owner' | 'existingPath'>): string {
  const app = /^(?:\/Users\/[^/]+|~)\/Library\/Fonts\/([^/]+)\//.exec(c.existingPath)?.[1];
  if (app) return t('act.owner.by', { app });
  switch (c.owner) {
    case 'system': return t('act.owner.system');
    case 'user_installed': return t('act.owner.user');
    case 'adobe_fonts': return t('act.owner.by', { app: 'Adobe Fonts' });
    case 'right_font': return t('act.owner.by', { app: 'RightFont' });
    case 'font_base': return t('act.owner.by', { app: 'FontBase' });
    case 'font_atlas': return t('act.owner.atlas');
    default: return t('act.owner.other');
  }
}

export function groupByFamily(items: readonly ConflictItem[]): ConflictGroup[] {
  const by = new Map<string, ConflictGroup>();
  for (const it of items) {
    const g = by.get(it.family) ?? { family: it.family, styles: [], conflicts: [] };
    if (!g.styles.includes(it.style)) g.styles.push(it.style);
    for (const c of it.conflicts) if (!g.conflicts.some(x => x.existingPath === c.existingPath && x.postscriptName === c.postscriptName)) g.conflicts.push(c);
    by.set(it.family, g);
  }
  return [...by.values()].sort((a, b) => a.family.localeCompare(b.family));
}

/** Nội dung tooltip của dấu "!": ai cài + đường dẫn (tối đa 3 bản, còn lại "+N"). Đường dẫn KHÔNG in inline. */
export function groupNote(g: ConflictGroup): string {
  const uniq = [...new Map(g.conflicts.map(c => [c.existingPath, c])).values()];
  const shown = uniq.slice(0, 3).map(c => `${ownerNote(c)} · ${c.existingPath}`);
  return uniq.length > 3 ? [...shown, t('act.conflict.tip.more', { n: uniq.length - 3 })].join('\n') : shown.join('\n');
}

/** Tên family/kiểu cho từng khoá face; thiếu trong thư viện thì dùng tên PostScript (bỏ phần sau dấu "-"). */
export function toItems(rows: readonly FaceRow[], keys: readonly FaceKeyDto[], conflicts: readonly ConflictDto[][]): ConflictItem[] {
  const byKey = new Map(rows.map(r => [`${r.hashHex}:${r.faceIndex}`, r]));
  return keys.map((k, i) => {
    const r = byKey.get(`${k.hashHex}:${k.faceIndex}`);
    const ps = conflicts[i]?.[0]?.postscriptName ?? '';
    const [fam, ...rest] = ps.split('-');
    return { family: r?.family ?? (fam || k.hashHex.slice(0, 8)), style: r?.subfamily ?? (rest.join('-') || 'Regular'), conflicts: conflicts[i] ?? [] };
  });
}
