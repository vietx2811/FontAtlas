import type { ComboRoleDto } from '../../bindings';
import type { PairRole } from '../../api';

export const ROLE_ORDER: ComboRoleDto[] = ['display', 'heading', 'subheading', 'body', 'caption', 'accent', 'quote'];
/** Nhóm "tiêu đề" (heading-like) và nhóm "nội dung" (body-like): ghép luôn là một font của nhóm này với một font của nhóm kia. */
export const HEADING_LIKE: ReadonlySet<ComboRoleDto> = new Set<ComboRoleDto>(['display', 'heading', 'subheading', 'accent', 'quote']);
export const isHeadingLike = (r: ComboRoleDto): boolean => HEADING_LIKE.has(r);
/** Vai của font neo khi tìm ứng viên cho `role`: cần font nhóm đối diện; `PairRole` là vai của font ĐẦU VÀO (neo). */
export const anchorPairRole = (role: ComboRoleDto): PairRole => (isHeadingLike(role) ? 'body' : 'heading');
/** Thứ tự ưu tiên chọn font neo trong nhóm đối diện. */
export const anchorRoles = (role: ComboRoleDto): ComboRoleDto[] => (isHeadingLike(role) ? ['body', 'caption'] : ['display', 'heading', 'subheading', 'accent', 'quote']);
