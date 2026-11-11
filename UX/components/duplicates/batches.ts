import type { ResolutionDto } from '../../api';

export type Batch = { ids: string[]; rows: ResolutionDto[] };
/** Một thao tác "Thùng rác" ra tối đa hai mẻ ở backend (tệp vào Thùng rác, rồi tệp chỉ bị ẩn). Mẻ Thùng rác và mẻ Ẩn kề nhau trong vòng này coi là một thao tác. */
export const SPLIT_BATCH_WINDOW_SEC = 60;

/** Gom theo mẻ (danh sách mới nhất trước). Hai mẻ kề nhau, một toàn Thùng rác và một toàn Ẩn, tạo cách nhau ≤ 60 giây → một dòng mẻ: Hoàn tác mẻ hoàn tác cả hai. */
export function groupBatches(list: ResolutionDto[]): Batch[] {
  const out: Batch[] = [];
  for (const r of list) { const b = out.find(x => x.ids.includes(r.batchId)); if (b) b.rows.push(r); else out.push({ ids: [r.batchId], rows: [r] }); }
  const kind = (b: Batch) => (b.rows.every(r => r.action === 'trash') ? 'trash' : b.rows.every(r => r.action === 'hide') ? 'hide' : 'mixed');
  const at = (b: Batch) => Math.max(...b.rows.map(r => r.createdAt));
  const merged: Batch[] = [];
  for (const b of out) {
    const prev = merged[merged.length - 1];
    if (prev && kind(prev) !== kind(b) && kind(prev) !== 'mixed' && kind(b) !== 'mixed' && Math.abs(at(prev) - at(b)) <= SPLIT_BATCH_WINDOW_SEC) {
      prev.ids.push(...b.ids); prev.rows = [...prev.rows, ...b.rows].sort((x, y) => y.id - x.id);
    } else merged.push({ ids: [...b.ids], rows: [...b.rows] });
  }
  return merged;
}
