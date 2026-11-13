import type { ConflictDto, FaceKeyDto, FaceRow, VfPolicyDto } from '../../api';
import { t } from '../../i18n';

/** Một font cần xác nhận: tên family/kiểu đọc từ thư viện, và các bản trùng đang cài ở máy. */
export type ConflictItem = { hashHex?: string; family: string; style: string; conflicts: ConflictDto[] };
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
    case 'typeface': return t('act.owner.by', { app: 'Typeface' });
    case 'suitcase': return t('act.owner.by', { app: 'Suitcase / Connect' });
    case 'font_atlas': return t('act.owner.atlas');
    default: return t('act.owner.other');
  }
}

/** "Thay thế" theo ma trận — backend tính (đường dẫn đã canonical hoá); UI dùng nguyên, không tự đoán từ đường dẫn. */
export type ReplaceTier = ConflictDto['replaceTier'];
export const replaceTierOf = (c: Pick<ConflictDto, 'replaceTier'>): ReplaceTier => c.replaceTier;

const APP_NAME: Partial<Record<ConflictDto['owner'], string>> = {
  right_font: 'RightFont', font_base: 'FontBase', typeface: 'Typeface', adobe_fonts: 'Creative Cloud (Adobe Fonts)', suitcase: 'Suitcase / Connect',
};

export type ReplaceAdvice = { tier: ReplaceTier; apps: string[]; unknownApp: boolean; protectedPath: string | null };

/** Gộp mọi xung đột của hộp thoại (cùng tập backend xét: KHÔNG bỏ sameFile): tier tệ nhất thắng (protected > guide > replace),
 *  tên các app đã nhận diện, và có bản "app lạ" không. */
export function replaceAdvice(items: readonly ConflictItem[]): ReplaceAdvice {
  const all = items.flatMap(i => i.conflicts);
  const rank: Record<ReplaceTier, number> = { replace: 0, guide: 1, protected: 2 };
  let tier: ReplaceTier = 'replace';
  const apps: string[] = [];
  let unknownApp = false;
  let protectedPath: string | null = null;
  for (const c of all) {
    const k = replaceTierOf(c);
    if (rank[k] > rank[tier]) tier = k;
    if (k === 'protected') protectedPath ??= c.existingPath;
    const app = APP_NAME[c.owner];
    if (app) { if (!apps.includes(app)) apps.push(app); } else if (c.owner === 'other') unknownApp = true;
  }
  return { tier, apps, unknownApp, protectedPath };
}

/** Xung đột người dùng đã thấy, gửi kèm "Thay bằng bản này": backend từ chối nếu tập này đã đổi (TOCTOU). */
export function confirmedOf(items: readonly ConflictItem[]): { hashHex: string; existingPath: string }[] {
  const out = new Map<string, { hashHex: string; existingPath: string }>();
  for (const it of items) if (it.hashHex) for (const c of it.conflicts) out.set(`${it.hashHex}\n${c.existingPath}`, { hashHex: it.hashHex, existingPath: c.existingPath });
  return [...out.values()];
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
    return { hashHex: k.hashHex, family: r?.family ?? (fam || k.hashHex.slice(0, 8)), style: r?.subfamily ?? (rest.join('-') || 'Regular'), conflicts: conflicts[i] ?? [] };
  });
}

/** G7: lựa chọn cho cặp variable (VF) ↔ static trùng tên PostScript. `keep` = dùng bản đang có (bỏ qua bản mới),
 *  `use` = dùng bản mới (tắt bản kia), `both` = giữ cả hai. Lựa chọn đã nhớ lưu theo LOẠI ("vf" | "static" | "both"), không theo bản. */
export type VfChoice = Exclude<VfPolicyDto, 'ask'>;
export type VfAction = 'keep' | 'use' | 'both';

/** Cả hộp thoại là các cặp VF↔static (mọi bản trùng khác loại với font sắp bật, mọi font sắp bật cùng loại)?
 *  Có bản trùng không đọc được loại (`existingVariable` null) hoặc cùng loại → hộp thoại xung đột thường. */
export function vfPairOf(items: readonly ConflictItem[]): { newIsVf: boolean } | null {
  const cs = items.flatMap(i => i.conflicts).filter(c => !c.sameFile);
  if (!cs.length) return null;
  const newIsVf = cs[0].variable;
  return cs.every(c => c.variable === newIsVf && c.existingVariable === !newIsVf) ? { newIsVf } : null;
}
export const vfActionOf = (choice: VfChoice, newIsVf: boolean): VfAction => (choice === 'both' ? 'both' : (choice === 'vf') === newIsVf ? 'use' : 'keep');
export const vfChoiceOf = (action: VfAction, newIsVf: boolean): VfChoice => (action === 'both' ? 'both' : (action === 'use') === newIsVf ? 'vf' : 'static');
