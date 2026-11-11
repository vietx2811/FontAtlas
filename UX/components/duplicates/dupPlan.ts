import type { PlanItemDto, PlanMemberDto, ResolveItemDto, ResolveReasonDto } from '../../api';

export const groupId = (kind: string, key: string): string => `${kind}\u0000${key}`;
export const itemId = (p: { kind: string; key: string }): string => groupId(p.kind, p.key);

export type PlanView = {
  keeper: PlanMemberDto; losers: PlanMemberDto[]; reason: ResolveReasonDto; chosen: boolean;
  /** Mọi bản của nhóm (bản giữ trước), để người dùng chọn lại. */
  all: PlanMemberDto[];
};

/** Áp lựa chọn "Đổi bản giữ" của người dùng lên kế hoạch của máy. `keeperPath` lạ hoặc rỗng → giữ nguyên kế hoạch. */
export function viewOf(item: PlanItemDto, keeperPath?: string | null): PlanView {
  const all = [item.keeper, ...item.losers];
  const pick = keeperPath && keeperPath !== item.keeper.path ? all.find(m => m.path === keeperPath) : undefined;
  if (!pick) return { keeper: item.keeper, losers: item.losers, reason: item.reason, chosen: false, all };
  return { keeper: pick, losers: all.filter(m => m !== pick), reason: { code: 'chosen_by_user', args: {} }, chosen: true, all: [pick, ...all.filter(m => m !== pick)] };
}

/** Một nhóm trong yêu cầu `resolve_apply`. `memberPaths` lấy nguyên văn từ kế hoạch người dùng đang xem. */
export function toRequestItem(item: PlanItemDto, v: PlanView, onlyLosers?: ReadonlySet<string>, hiddenAlready?: ReadonlySet<string>): ResolveItemDto {
  const losers = v.losers.map(l => l.path).filter(p => !onlyLosers || onlyLosers.has(p));
  const members = hiddenAlready?.size ? item.memberPaths.filter(p => !hiddenAlready.has(p)) : item.memberPaths;
  return { kind: item.kind, key: item.key, keeperPath: v.keeper.path, loserPaths: losers, memberPaths: members };
}

export type TrashState = 'ok' | 'explicit' | 'blocked';
export const trashState = (m: PlanMemberDto): TrashState => (!m.trash.allowed ? 'blocked' : m.trash.needsExplicit ? 'explicit' : 'ok');

/** Chia các bản thua theo cách xử lý khi người dùng chọn Thùng rác: vào Thùng rác (đủ điều kiện, đã tick nếu cần) hoặc chỉ ẩn. */
export function splitForTrash(losers: PlanMemberDto[], ticked: ReadonlySet<string>): { trash: PlanMemberDto[]; hideOnly: PlanMemberDto[] } {
  const trash: PlanMemberDto[] = [], hideOnly: PlanMemberDto[] = [];
  for (const l of losers) { const s = trashState(l); (s === 'ok' || (s === 'explicit' && ticked.has(l.path)) ? trash : hideOnly).push(l); }
  return { trash, hideOnly };
}

export const baseName = (p: string): string => p.slice(p.lastIndexOf('/') + 1);
export const memberName = (m: { family: string; style: string | null }): string => `${m.family}${m.style ? ` ${m.style}` : ''}`;

/** Mã lỗi "thành công một phần": quyết định ĐÃ ghi nhưng còn việc dở. */
export const PARTIAL_CODES: ReadonlySet<string> = new Set(['rollback_failed', 'reactivate_failed', 'keeper_activation_failed', 'keeper_deactivate_failed']);
