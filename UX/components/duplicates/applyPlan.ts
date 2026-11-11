import type { Api, PlanItemDto, ResolveRequestDto, ResolveResultDto } from '../../api';
import { splitForTrash, toRequestItem, viewOf } from './dupPlan';

export type ApplyInput = {
  api: Api; items: PlanItemDto[]; /** Bản giữ người dùng đã chọn lại: itemId → đường dẫn. */ choices: Record<string, string>;
  action: 'hide' | 'trash'; /** Tệp ở nơi cần xác nhận riêng mà người dùng đã tick. */ ticked: ReadonlySet<string>;
  /** Nút "Giải quyết tất cả an toàn" (và không có lựa chọn nào bị đổi). */ allSafe: boolean;
};

const empty = (batchId = ''): ResolveResultDto => ({ batchId, done: [], failed: [], needsRescan: false });
const merge = (a: ResolveResultDto, b: ResolveResultDto): ResolveResultDto => ({ batchId: a.batchId || b.batchId, done: [...a.done, ...b.done], failed: [...a.failed, ...b.failed], needsRescan: a.needsRescan || b.needsRescan });

/**
 * Áp dụng kế hoạch người dùng đang xem.
 * - Ẩn: một yêu cầu. "Giải quyết tất cả an toàn" gửi `mode: 'all_safe'` (backend tính lại quy tắc từng nhóm).
 * - Thùng rác: tệp đủ điều kiện (và đã tick nếu cần) vào Thùng rác; các tệp còn lại CHỈ bị ẩn. Hai yêu cầu liên tiếp;
 *   yêu cầu thứ hai bỏ khỏi `memberPaths` những đường dẫn đã xử lý ở yêu cầu đầu, vì với backend đó không còn là thành viên của nhóm.
 */
export async function applyPlan(i: ApplyInput): Promise<ResolveResultDto> {
  const views = i.items.map(it => ({ it, v: viewOf(it, i.choices[`${it.kind}\u0000${it.key}`]) }));
  if (i.action === 'hide') {
    const items = views.map(({ it, v }) => toRequestItem(it, v));
    return i.api.resolveApply({ items, action: 'hide', explicitTrashPaths: [], mode: i.allSafe ? 'all_safe' : 'chosen' });
  }
  const trashSet = new Set<string>(), hideSet = new Set<string>();
  for (const { v } of views) { const s = splitForTrash(v.losers, i.ticked); s.trash.forEach(m => trashSet.add(m.path)); s.hideOnly.forEach(m => hideSet.add(m.path)); }
  let out = empty();
  const handled = new Set<string>();
  const run = async (action: 'hide' | 'trash', only: ReadonlySet<string>): Promise<void> => {
    if (only.size === 0) return;
    const items = views.map(({ it, v }) => toRequestItem(it, v, only, handled)).filter(x => x.loserPaths.length > 0);
    if (items.length === 0) return;
    const req: ResolveRequestDto = { items, action, explicitTrashPaths: action === 'trash' ? [...only].filter(p => i.ticked.has(p)) : [], mode: 'chosen' };
    const r = await i.api.resolveApply(req);
    r.done.forEach(d => handled.add(d.loserPath));
    out = merge(out, r);
  };
  await run('trash', trashSet);
  await run('hide', hideSet);
  return out;
}

/** Số nhóm khác nhau trong các quyết định đã ghi. */
export const groupsDone = (r: ResolveResultDto): number => new Set(r.done.map(d => `${d.kind}\u0000${d.groupKey}`)).size;
