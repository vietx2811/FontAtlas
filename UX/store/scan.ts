// Theo dõi quét nguồn: báo "đang quét", và buộc lưới/sidebar/tiêu đề tải lại khi có font mới (không có chuyện UI hiện 0 sau khi quét xong).
import { create, type StoreApi, type UseBoundStore } from 'zustand';
import type { Api } from '../api';
import { openPrivacySettings } from '../privacy';
import { useNoticeStore } from './notices';

export const BUMP_THROTTLE_MS = 1500;

export type AccessProblem = 'denied' | 'timeout';
type State = {
  scanning: boolean; written: number; paused: boolean; version: number; access: Record<number, AccessProblem>;
  /** Số file chỉ là placeholder đám mây (chưa tải về) đã bỏ qua trong lần quét đang chạy, theo nguồn. */
  cloud: Record<number, number>;
  cloudTotal(): number;
  bump(): void;
};
export type ScanStore = UseBoundStore<StoreApi<State>>;

export function createScanStore(): ScanStore {
  return create<State>()((set, get) => ({
    scanning: false, written: 0, paused: false, version: 0, access: {}, cloud: {},
    cloudTotal: () => Object.values(get().cloud).reduce((a, b) => a + b, 0),
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
    const clear = (id: number) => set(s => { const { [id]: _drop, ...rest } = s.access; return { access: rest }; });
    const dropCloud = (id: number) => set(s => { const { [id]: _d, ...rest } = s.cloud; return { cloud: rest }; });
    if (e.type === 'queued') { set({ scanning: true }); clear(e.sourceId); dropCloud(e.sourceId); }
    else if (e.type === 'progress') set(s => ({ scanning: true, written: e.progress.facesWritten, cloud: { ...s.cloud, [e.sourceId]: e.progress.notDownloaded } }));
    else if (e.type === 'facesAdded') bumpSoon();
    else if (e.type === 'paused') set({ paused: true });
    else if (e.type === 'resumed') set({ paused: false });
    else if (e.type === 'finished') { clear(e.sourceId); dropCloud(e.sourceId); bumpNow(); } // từ đây số chưa tải nằm trong lần quét cuối của nguồn
    else if (e.type === 'failed') {
      set({ scanning: false });
      const d = e.error.detail as { source?: string; op?: string; reason?: string } | null;
      if (d?.source === 'app' && d.op === 'opendir' && (d.reason === 'timeout' || d.reason === 'permission_denied')) {
        // Không đọc được thư mục vì chưa có quyền: font đã quét giữ nguyên, chỉ đánh dấu nguồn cần quyền.
        set(s => ({ access: { ...s.access, [e.sourceId]: d.reason === 'timeout' ? 'timeout' : 'denied' } }));
        void api.listSources().then(list => list.find(x => x.id === e.sourceId)?.name ?? 'nguồn này', () => 'nguồn này').then(name =>
          useNoticeStore.getState().push({
            kind: 'error', text: `Không đọc được “${name}”: cần quyền truy cập thư mục này.`,
            action: { label: 'Cấp quyền', run: () => { void openPrivacySettings(); } },
          }));
      } else useNoticeStore.getState().push({ kind: 'error', text: `Quét nguồn lỗi (${e.error.code}): ${e.error.message}` });
    } else if (e.type === 'idle') { set({ scanning: false, written: 0 }); if (Date.now() - last > 200) bumpNow(); } // `finished` vừa tải lại rồi
  });
  try {
    const st = await api.scanStatus();
    if (st.running) store.setState({ scanning: true, paused: st.paused });
  } catch (e) { console.warn('scan_status failed', e); }
  return () => { off(); clearTimeout(timer); };
}
