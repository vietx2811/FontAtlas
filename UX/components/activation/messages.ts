import type { AutoDeactivatedDto, FileOutcomeDto, ReconcileReportDto } from '../../api';

export type Msg = { kind: 'info' | 'error'; text: string };

/** Thông báo cho một kết quả kích hoạt; thành công thì không nói gì (null). */
export function describeOutcome(o: FileOutcomeDto): Msg | null {
  switch (o.kind) {
    case 'failed':
      if (o.code === 'file_changed') return { kind: 'error', text: 'File font đã thay đổi trên đĩa sau lần quét gần nhất. Hãy quét lại nguồn rồi thử lại.' };
      return { kind: 'error', text: `Không kích hoạt được (${o.code}): ${o.message}` };
    case 'missing_file': return { kind: 'error', text: 'Không tìm thấy file (ổ ngoài đã rút hoặc file đã chuyển).' };
    case 'not_downloaded': return { kind: 'error', text: 'File đang ở iCloud, chưa tải về máy.' };
    case 'busy': return { kind: 'info', text: 'Font đang được xử lý, hãy thử lại sau giây lát.' };
    default: return null;
  }
}

const CAUSE: Record<string, string> = {
  collection_deleted: 'vì dự án đã bị xoá',
  left_collection: 'vì đã rời khỏi dự án',
  no_reason: 'vì không còn lý do kích hoạt',
};
const group = (items: AutoDeactivatedDto[]) => {
  const m = new Map<string, number>();
  for (const i of items) m.set(i.cause, (m.get(i.cause) ?? 0) + 1);
  return m;
};

/** Thông báo sau `reconcile` (lúc mở app hoặc khi dự án đổi). Không có gì xảy ra thì trả mảng rỗng. */
export function reconcileNotices(r: ReconcileReportDto): Msg[] {
  const out: Msg[] = [];
  for (const [cause, n] of group(r.autoDeactivated)) out.push({ kind: 'info', text: `Đã tự tắt ${n} font ${CAUSE[cause] ?? `(${cause})`}.` });
  if (r.autoDeactivateFailed.length) {
    out.push({ kind: 'error', text: `Không gỡ được kích hoạt ${r.autoDeactivateFailed.length} font (hệ điều hành từ chối); font vẫn đang bật.` });
  }
  const parts = [
    r.reregistered > 0 && `Đã kích hoạt lại ${r.reregistered} font.`,
    r.missing > 0 && `${r.missing} font không tìm thấy file.`,
    r.notDownloaded > 0 && `${r.notDownloaded} font còn ở iCloud, chưa tải về máy.`,
  ].filter(Boolean) as string[];
  if (parts.length) out.push({ kind: 'info', text: parts.join(' ') });
  return out;
}
