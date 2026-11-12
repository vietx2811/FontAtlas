// "Tải về máy" theo nguồn: theo dõi tiến độ để Sidebar / Settings / menu cùng một trạng thái, và báo kết quả bằng thông báo gọn.
import { create, type StoreApi, type UseBoundStore } from 'zustand';
import type { Api } from '../api';
import { CmdError, type CloudDownloadReportDto } from '../api/types';
import { useNoticeStore, type NoticeStore } from './notices';
import { t, tn, hasKey } from '../i18n';

export type CloudRun = { done: number; total: number; failed: number; cancelling: boolean };
type State = { runs: Record<number, CloudRun>; set(id: number, r: CloudRun | null): void };
export type CloudDownloadStore = UseBoundStore<StoreApi<State>>;

export function createCloudDownloadStore(): CloudDownloadStore {
  return create<State>()(set => ({
    runs: {},
    set: (id, r) => set(s => {
      const { [id]: _drop, ...rest } = s.runs;
      return { runs: r ? { ...rest, [id]: r } : rest };
    }),
  }));
}
export const useCloudDownloadStore = createCloudDownloadStore();

/** Lý do (theo mã backend) bằng lời dễ hiểu; mã lạ → câu chung. */
export function whyText(code: string | null | undefined): string {
  const key = `cdl.why.${code ?? ''}`;
  return code && hasKey(key) ? t(key) : t('cdl.why.other');
}

/** Hàm thuần: báo cáo → thông báo. Thành công thuần = info; có lỗi / huỷ dở = error để người dùng thấy. */
export function cloudDownloadNotice(r: CloudDownloadReportDto, name: string): { kind: 'info' | 'error'; text: string } {
  if (r.total === 0) return { kind: 'info', text: t('cdl.none', { name }) };
  if (r.cancelled) return { kind: 'info', text: t('cdl.cancelled', { ok: r.downloaded, total: r.total, name }) };
  if (r.downloaded === 0) return { kind: 'error', text: t('cdl.failAll', { name, why: whyText(r.firstFailureCode) }) };
  if (r.failed > 0) return { kind: 'error', text: t('cdl.partial', { ok: r.downloaded, bad: r.failed, name, why: whyText(r.firstFailureCode) }) };
  return { kind: 'info', text: tn('cdl.done', { n: r.downloaded, name }) };
}

/** Bắt đầu tải cho nguồn. Không ném: mọi kết quả (kể cả lỗi) thành thông báo; nguồn đang tải thì bỏ qua. */
export async function downloadSourceToMac(
  api: Api, src: { id: number; name: string },
  store: CloudDownloadStore = useCloudDownloadStore, notices: NoticeStore = useNoticeStore,
): Promise<CloudDownloadReportDto | null> {
  if (store.getState().runs[src.id]) return null;
  store.getState().set(src.id, { done: 0, total: 0, failed: 0, cancelling: false });
  try {
    const rep = await api.sourceDownloadCloud(src.id, p => {
      const cur = store.getState().runs[src.id];
      store.getState().set(src.id, { done: p.done, total: p.total, failed: p.failed, cancelling: cur?.cancelling ?? false });
    });
    notices.getState().push(cloudDownloadNotice(rep, src.name));
    return rep;
  } catch (e) {
    const msg = e instanceof CmdError ? e.message : (e as Error)?.message ?? String(e);
    notices.getState().push({ kind: 'error', text: t('cdl.error', { name: src.name, msg }) });
    return null;
  } finally {
    store.getState().set(src.id, null);
  }
}

/** Huỷ lần tải đang chạy của nguồn (nút Huỷ). */
export async function cancelSourceDownload(api: Api, id: number, store: CloudDownloadStore = useCloudDownloadStore): Promise<void> {
  const cur = store.getState().runs[id];
  if (!cur) return;
  store.getState().set(id, { ...cur, cancelling: true });
  await api.sourceDownloadCancel(id).catch(() => false);
}
