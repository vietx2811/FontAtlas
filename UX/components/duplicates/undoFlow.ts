import type { Api, ResolveResultDto } from '../../api';
import { t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { useUiStore } from '../../store/ui';
import { failText, failWhy } from './dupText';

/** Quá số này (mã lỗi khác nhau) thì không xếp một chồng toast: mở hộp kết quả. */
export const MAX_ERROR_TOASTS = 3;

/**
 * Hoàn tác + phản hồi dùng chung (toast "Hoàn tác" và tab Đã giải quyết): lỗi gộp theo mã ("N tệp: lý do"), tăng libraryRev,
 * quét lại nếu tệp đã về từ Thùng rác. Nhiều loại lỗi → `onReport` mở hộp kết quả (chi tiết thô nằm sau "Chi tiết").
 */
export async function undoWithFeedback(api: Api, ids: number[], onReport?: (r: ResolveResultDto) => void): Promise<ResolveResultDto> {
  const notices = useNoticeStore.getState();
  try {
    const r = await api.resolveUndo(ids);
    if (r.done.length > 0) { useUiStore.getState().bumpLibrary(true); notices.push({ kind: 'info', text: t('dup.res.undone', { n: r.done.length }) }); }
    const byCode = new Map<string, typeof r.failed>();
    for (const f of r.failed) byCode.set(f.code, [...(byCode.get(f.code) ?? []), f]);
    if (byCode.size > MAX_ERROR_TOASTS && onReport) onReport(r);
    else for (const list of byCode.values()) notices.push({ kind: 'error', text: list.length === 1 ? failText(list[0]) : t('dup.res.failMany', { n: list.length, why: failWhy(list[0].code) }) });
    if (r.needsRescan) { notices.push({ kind: 'info', text: t('dup.res.rescan') }); void api.rescanSources().catch(() => undefined); }
    return r;
  } catch (e) {
    notices.push({ kind: 'error', text: t('dup.fail.unknown', { name: e instanceof Error ? e.message : String(e) }) });
    return { batchId: '', done: [], failed: [], needsRescan: false };
  }
}
