import type { ComboRoleDto } from '../../bindings';
import type { PairRole } from '../../api';
import { t } from '../../i18n';

export const ROLE_ORDER: ComboRoleDto[] = ['display', 'heading', 'subheading', 'body', 'caption', 'accent', 'quote'];
/** Nhóm "tiêu đề" (heading-like) và nhóm "nội dung" (body-like): ghép luôn là một font của nhóm này với một font của nhóm kia. */
export const HEADING_LIKE: ReadonlySet<ComboRoleDto> = new Set<ComboRoleDto>(['display', 'heading', 'subheading', 'accent', 'quote']);
/** Tên vai gợi sẵn trong "+ Role" (G3). Vai tuỳ ý thì gõ tên riêng. `label`/`numerals` thuộc nhóm nội dung. */
export const SUGGESTED_ROLES: readonly string[] = [...ROLE_ORDER, 'label', 'numerals'];
/** Khoá preset của một tên vai (không phân biệt hoa thường, nhận cả tên đã dịch: "Body"/"Nội dung"); không phải preset → null. */
export const presetKey = (r: string): string | null => {
  const k = r.trim().toLowerCase();
  return SUGGESTED_ROLES.find(p => p === k || t(`role.${p}` as 'role.display').toLowerCase() === k) ?? null;
};
/** Tên hiển thị: preset dịch theo locale, vai tuỳ ý giữ nguyên chữ người dùng đặt. */
export const roleLabel = (r: string): string => { const k = presetKey(r); return k ? t(`role.${k}` as 'role.display') : r; };
/** Giải thích ngắn của preset (TermTip/tooltip); vai tuỳ ý không có. */
export const roleGloss = (r: string): string | null => { const k = presetKey(r); return k ? t(`role.${k}.gloss` as 'role.display.gloss') : null; };
/** Vai tuỳ ý mặc định coi là vai nội dung (ghép với tiêu đề) cho tới khi có hồ sơ vai (G5). */
export const isHeadingLike = (r: ComboRoleDto): boolean => HEADING_LIKE.has(presetKey(r) ?? '');
/** Vai của font neo khi tìm ứng viên cho `role`: cần font nhóm đối diện; `PairRole` là vai của font ĐẦU VÀO (neo). */
export const anchorPairRole = (role: ComboRoleDto): PairRole => (isHeadingLike(role) ? 'body' : 'heading');
/** Thứ tự ưu tiên chọn font neo trong nhóm đối diện. */
export const anchorRoles = (role: ComboRoleDto): ComboRoleDto[] => (isHeadingLike(role) ? ['body', 'caption'] : ['display', 'heading', 'subheading', 'accent', 'quote']);
