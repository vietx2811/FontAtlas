import type { ResolutionDto } from '../../api';

export type Batch = { ids: string[]; rows: ResolutionDto[] };

/** Gom theo `batchId` (danh sách mới nhất trước). Một thao tác gồm nhiều request (Thùng rác rồi chỉ-ẩn) dùng `joinBatch` nên đã chung một mẻ ở backend. */
export function groupBatches(list: ResolutionDto[]): Batch[] {
  const out: Batch[] = [];
  for (const r of list) { const b = out.find(x => x.ids[0] === r.batchId); if (b) b.rows.push(r); else out.push({ ids: [r.batchId], rows: [r] }); }
  return out;
}
