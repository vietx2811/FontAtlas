import type { ActivationReportDto, FaceKeyDto } from '../../api';
import { t, tn } from '../../i18n';
import { useNoticeStore } from '../../store/notices';

/**
 * Font "đang bận" (một lượt khác đang đụng cùng file): không đổi gì và sẽ thử lại được.
 * Ưu tiên số `busy` của báo cáo (backend 02); chưa có thì đếm kết quả `busy` theo file.
 */
export function busyFaces(rep: Pick<ActivationReportDto, 'results'>): FaceKeyDto[] {
  return rep.results.filter(r => r.outcome.kind === 'busy').flatMap(r => (r.requestedFaces.length ? r.requestedFaces : [0]).map(faceIndex => ({ hashHex: r.hashHex, faceIndex })));
}
export function busyCount(rep: ActivationReportDto): number {
  const field = (rep as { busy?: number }).busy;
  return typeof field === 'number' ? field : new Set(busyFaces(rep).map(f => f.hashHex)).size;
}

/**
 * Sau Tắt hết / đổi scope / chuyển font: nếu có font bận thì báo "N font đang bận, sẽ thử lại" kèm nút [Thử lại] chạy lại ĐÚNG những font đó.
 * Trả true nếu đã báo. `retry` nhận danh sách font bận và tự xử lý kết quả của lần thử (thường gọi lại chính hàm này).
 */
export function noticeBusy(rep: ActivationReportDto, retry: (faces: FaceKeyDto[]) => void | Promise<unknown>): boolean {
  const n = busyCount(rep);
  if (n <= 0) return false;
  const faces = busyFaces(rep);
  useNoticeStore.getState().push({
    kind: 'info', text: tn('act.busy', { n }),
    action: faces.length ? { label: t('act.busy.retry'), run: () => void retry(faces) } : undefined,
  });
  return true;
}
