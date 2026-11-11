import type { Api } from '../../api';
import { t } from '../../i18n';
import { refreshActivation } from '../../store/activationSync';
import { useNoticeStore } from '../../store/notices';

/** "Khôi phục bản cũ" cho các dòng sổ `ids`; nói thật kết quả (file mất, còn trên cloud, lỗi). Dùng cho thông báo và Cài đặt. */
export async function restoreReplaced(api: Api, ids: number[]): Promise<boolean> {
  const push = useNoticeStore.getState().push;
  let ok = false;
  try {
    const outs = [];
    for (const id of ids) outs.push(await api.activationRestoreReplaced(id));
    if (outs.includes('missing_file')) push({ kind: 'error', text: t('act.replace.restoreMissing') });
    else if (outs.includes('not_downloaded')) push({ kind: 'error', text: t('act.replace.restoreNotDownloaded') });
    else { ok = true; push({ kind: 'info', text: t('act.replace.restored') }); }
  } catch (e) {
    push({ kind: 'error', text: t('act.fail', { msg: (e as Error).message }) });
  }
  await refreshActivation(api);
  return ok;
}
