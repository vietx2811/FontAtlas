// Theo dõi quét nguồn: báo "đang quét", và buộc lưới/sidebar/tiêu đề tải lại khi có font mới (không có chuyện UI hiện 0 sau khi quét xong).
import { create, type StoreApi, type UseBoundStore } from 'zustand';
import type { Api } from '../api';
import { useNoticeStore } from './notices';

export const BUMP_THROTTLE_MS = 1500;

type State = { scanning: boolean; written: number; paused: boolean; version: number; bump(): void };
export type ScanStore = UseBoundStore<StoreApi<State>>;

export function createScanStore(): ScanStore {
  return create<State>()(set => ({
    scanning: false, written: 0, paused: false, version: 0,
    bump: () => set(s => ({ version: s.version + 1 })),
  }));
}
export const useScanStore = createScanStore();

export async function startScanSync(api: Api, store: ScanStore = useScanStore): Promise<() => void> {
  let last = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const bumpNow = () => { clearTimeout(timer); timer = undefined; last = Date.now(); store.getState().bump(); };
  const bumpSoon = () => {
    const wait = BUMP_THROTTLE_MS - (Date.now() - last);
    if (wait <= 0) bumpNow(); else if (!timer) timer = setTimeout(bumpNow, wait);
  };
  const off = await api.scanSubscribe(e => {
    const set = store.setState;
    if (e.type === 'queued') set({ scanning: true });
    else if (e.type === 'progress') set({ scanning: true, written: e.progress.facesWritten });
    else if (e.type === 'facesAdded') bumpSoon();
    else if (e.type === 'paused') set({ paused: true });
    else if (e.type === 'resumed') set({ paused: false });
    else if (e.type === 'finished') bumpNow();
    else if (e.type === 'failed') {
      set({ scanning: false });
      useNoticeStore.getState().push({ kind: 'error', text: `Quét nguồn lỗi (${e.error.code}): ${e.error.message}` });
    } else if (e.type === 'idle') { set({ scanning: false, written: 0 }); if (Date.now() - last > 200) bumpNow(); } // `finished` vừa tải lại rồi
  });
  try {
    const st = await api.scanStatus();
    if (st.running) store.setState({ scanning: true, paused: st.paused });
  } catch (e) { console.warn('scan_status failed', e); }
  return () => { off(); clearTimeout(timer); };
}
