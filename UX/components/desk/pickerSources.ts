// Nguồn của Picker (G5): Pairs · Role fit · Similar · Pinned · Recent. Mỗi nguồn trả cùng một dạng `PickItem`.
import type { Api, FaceKeyDto } from '../../api';
import type { ComboRoleDto, FaceRowDto, FitDto, RoleRefDto, WhyDto } from '../../bindings';
import { t, hasKey, getLocale } from '../../i18n';
import { anchorPairRole } from './deskRoles';

export const SOURCES = ['pair', 'role', 'sim', 'pin', 'recent'] as const;
export type PickSource = typeof SOURCES[number];
export type PickItem = { face: FaceKeyDto; row: FaceRowDto; /** Câu "vì sao" đã dịch (≤ 1 dòng). */ summary: string; fit?: FitDto | null; pinned?: boolean; fromPeek?: boolean;
  /** Nguồn Ghép (G4): phong cách pair của dòng, mức hợp phong cách, cổng (safe/controlled/experimental). Chỉ nhãn, không số. */ styles?: string[]; styleFit?: FitDto; gate?: string };
export const PROFILES = ['body', 'caption', 'heading', 'display', 'quote', 'label', 'numerals'] as const;
export type Profile = typeof PROFILES[number];

/** Bảng đoán hồ sơ cho vai tuỳ ý (Phụ lục A spec G5): chỉ để ĐOÁN SẴN, người dùng thấy và đổi được; tên lạ → null. */
const GUESS: Array<[RegExp, Profile]> = [
  [/^(sub-?head(ing)?|tagline|title|tiêu đề phụ)$/i, 'heading'], [/^(eyebrow|tag|button|nút|nhãn)$/i, 'label'], [/^(price|giá|số|number)$/i, 'numerals'],
  [/^(pull ?quote|trích dẫn)$/i, 'quote'], [/^(note|footnote|chú thích)$/i, 'caption'], [/^(hero|headline|tiêu đề lớn)$/i, 'display'], [/^(paragraph|nội dung|text)$/i, 'body'],
];
export const guessProfile = (name: string): Profile | null => GUESS.find(([re]) => re.test(name.trim()))?.[1] ?? null;

export type RoleCtx = { role: ComboRoleDto; profile: Profile | null };
/** Vai gửi backend: tên người dùng; vai lạ kèm hồ sơ đã chọn (không có → backend trả fit unknown + role_unknown). */
export const roleRef = (c: RoleCtx): RoleRefDto => ({ name: c.role, profile: c.profile, size: null });

const CLAMP = 100;
/** Vai → RoleRef cho backend khi nhận ra (preset hoặc có hồ sơ); vai lạ → null (nguồn Ghép lùi về pair_candidates). */
function knownRole(name: string, profile: Profile | null = null): RoleRefDto | null {
  const k = name.trim().toLowerCase();
  if (PRESET.includes(k)) return { name: k, profile: null, size: null };
  const p = profile ?? guessProfile(name);
  return p ? { name, profile: p, size: null } : null;
}
const PRESET = ['display', 'heading', 'subheading', 'body', 'caption', 'accent', 'quote', 'label', 'numerals'];
export type LoadCtx = { /** Vai của font neo (nguồn Ghép). */ anchorRole?: ComboRoleDto | null; /** Phong cách pair đang chọn; '' = Tự động (mọi phong cách, xếp theo mức). */ style?: string; role: ComboRoleDto; profile: Profile | null; anchor: FaceKeyDto | null; current: FaceKeyDto | null; caseId: number; projectVi: boolean; mode: 'safe' | 'controlled' | 'experimental' | string };

export async function loadSource(api: Api, src: PickSource, c: LoadCtx): Promise<PickItem[]> {
  const target = roleRef(c);
  if (src === 'pair') {
    if (!c.anchor) return [];
    const aRef = c.anchorRole ? knownRole(c.anchorRole) : null; const tRef = knownRole(c.role, c.profile);
    if (aRef && tRef) {
      // Ghép theo vai + phong cách pair (G4). Cổng theo chế độ của Case: safe = chỉ an toàn; controlled = + có kiểm soát; còn lại mọi cổng.
      const page = await api.suggestPartners(c.anchor, { anchorRole: aRef, targetRole: tRef, styles: c.style ? [c.style] : [], projectVi: c.projectVi, limit: CLAMP, offset: 0, filter: null });
      const allow = c.mode === 'safe' ? ['safe'] : c.mode === 'controlled' ? ['safe', 'controlled'] : ['safe', 'controlled', 'experimental'];
      return page.items.filter(p => allow.includes(p.gate)).map(p => ({ face: p.face, row: p.row, fit: p.roleFit, styles: p.styles, styleFit: p.styleFit, gate: p.gate, summary: [...p.reasons, ...p.warnings].map(whyText).filter(Boolean).slice(0, 2).join(' · ') }));
    }
    const r = await api.pairCandidates(c.anchor, { role: anchorPairRole(c.role), mode: c.mode as never, projectVi: c.projectVi, k: 20, collectionId: null, lang: getLocale() });
    return r.filter(x => !(x.face.hashHex === c.anchor!.hashHex && x.face.faceIndex === c.anchor!.faceIndex)).map(x => ({ face: x.face, row: x.row, summary: x.summary }));
  }
  if (src === 'recent') {
    const r = await api.recentViews(60);
    return r.filter(v => v.row && !v.row.missing).map(v => ({ face: v.face, row: v.row!, summary: '' }));
  }
  const page = await api.deskSuggest({ source: src === 'role' ? 'fits_role' : src === 'sim' ? 'similar' : 'pinned', anchor: src === 'sim' ? c.current : src === 'role' ? c.anchor : null, // Role fit loại chính font ở vai đối diện (không tự ghép với chính nó)
     caseId: src === 'pin' ? c.caseId : null, targetRole: target, filter: null, projectVi: c.projectVi, limit: CLAMP, offset: 0 });
  return page.items.map(i => ({ face: i.face, row: i.row, fit: i.fit, pinned: i.pinned, summary: i.reasons.map(whyText).filter(Boolean).slice(0, 2).join(' · ') }));
}

/** Giữ các font thuộc `classes` (OR). Lọc phía backend bằng query_faces({faceKeys, classes}) vì dòng không mang class. */
export async function keepClasses(api: Api, items: readonly PickItem[], classes: readonly string[]): Promise<Set<string>> {
  if (!classes.length || !items.length) return new Set(items.map(i => `${i.face.hashHex}:${i.face.faceIndex}`));
  const r = await api.queryFaces({ faceKeys: items.map(i => i.face), classes: [...classes], limit: items.length });
  return new Set(r.items.map(x => `${x.hashHex}:${x.faceIndex}`));
}

/** Mã lý do của backend → câu vi/en. Tham số số KHÔNG in ra như điểm (Lead); mã chưa có bản dịch → bỏ dòng, không hiện mã thô. */
export function whyText(w: WhyDto): string {
  const code = w.code.startsWith('similar_') ? 'similar_axis' : w.code;
  // Mã chưa có câu riêng thì thử nhóm gốc (vd `spacing_tight` → `why.spacing`); không có nữa → bỏ dòng, không hiện mã thô.
  const key = hasKey(`why.${code}`) ? `why.${code}` : `why.${code.replace(/_[^_]+$/, '')}`;
  if (!hasKey(key)) return '';
  const axis = w.code.startsWith('similar_') ? w.code.slice('similar_'.length) : '';
  return (t as (k: string, p?: Record<string, string | number>) => string)(key, { axis, count: w.params.find(p => p.key === 'count')?.value ?? 0 });
}
