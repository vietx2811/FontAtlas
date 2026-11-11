import type { Api, ResolveResultDto } from '../../api';
import { t } from '../../i18n';
import { useNoticeStore } from '../../store/notices';
import { useUiStore } from '../../store/ui';
import { failText } from './dupText';

/** Hoàn tác + phản hồi dùng chung (toast "Hoàn tác" và tab Đã giải quyết): lỗi báo rõ theo mã, tăng libraryRev, quét lại nếu tệp đã về từ Thùng rác. */
export async function undoWithFeedback(api: Api, ids: number[]): Promise<ResolveResultDto> {
  const notices = useNoticeStore.getState();
  try {
    const r = await api.resolveUndo(ids);
    if (r.done.length > 0) { useUiStore.getState().bumpLibrary(true); notices.push({ kind: 'info', text: t('dup.res.undone', { n: r.done.length }) }); }
    for (const f of r.failed) notices.push({ kind: 'error', text: failText(f) });
    if (r.needsRescan) { notices.push({ kind: 'info', text: t('dup.res.rescan') }); void api.rescanSources().catch(() => undefined); }
    return r;
  } catch (e) {
    notices.push({ kind: 'error', text: t('dup.fail.unknown', { name: e instanceof Error ? e.message : String(e) }) });
    return { batchId: '', done: [], failed: [], needsRescan: false };
  }
}
