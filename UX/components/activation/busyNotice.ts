import type { ActivationReportDto, FaceKeyDto } from '../../api';
import { t, tn } from '../../i18n';
import { useNoticeStore } from '../../store/notices';

/**
 * Font "đang bận" (một lượt khác đang đụng cùng file): không đổi gì và thử lại được.
 * Số liệu và danh sách font cùng MỘT nguồn (kết quả theo file) để số trong câu khớp nút Thử lại; chỉ khi báo cáo không có kết quả theo file
 * (backend cắt bớt) mới dùng `report.busy`.
 */
export function busyFaces(rep: Pick<ActivationReportDto, 'results'>): FaceKeyDto[] {
  return rep.results.filter(r => r.outcome.kind === 'busy').flatMap(r => (r.requestedFaces.length ? r.requestedFaces : [0]).map(faceIndex => ({ hashHex: r.hashHex, faceIndex })));
}
export function busyN(rep: ActivationReportDto): number {
  const fromResults = new Set(busyFaces(rep).map(f => f.hashHex)).size;
  if (fromResults > 0) return fromResults;
  const field = (rep as { busy?: number }).busy;
  return typeof field === 'number' && field > 0 ? field : 0;
}
/** Câu "N font đang bận, sẽ thử lại" để GHÉP vào câu kết quả chính (một thông báo, không đôi); null nếu không có font bận. */
export const busyPart = (rep: ActivationReportDto): string | null => { const n = busyN(rep); return n > 0 ? tn('act.busy', { n }) : null; };

/**
 * Thao tác không có câu kết quả riêng (đổi scope, Tắt hết ở Cài đặt): chỉ báo khi có font bận, kèm [Thử lại] chạy lại ĐÚNG thao tác gốc.
 * `retried`: đây là lần thử lại → hết bận thì nói một câu ngắn thay vì im lặng; không bao giờ đẩy tóm tắt rỗng.
 */
export function pushBusyOnly(rep: ActivationReportDto, retry: () => void, retried = false): void {
  const part = busyPart(rep);
  const push = useNoticeStore.getState().push;
  if (part) push({ kind: 'info', text: part, action: { label: t('act.busy.retry'), run: retry } });
  else if (retried) push({ kind: 'info', text: t('act.busy.cleared') });
}
