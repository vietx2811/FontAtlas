// Theo dõi quét nguồn: báo "đang quét", và buộc lưới/sidebar/tiêu đề tải lại khi có font mới (không có chuyện UI hiện 0 sau khi quét xong).
import { create, type StoreApi, type UseBoundStore } from 'zustand';
import type { Api } from '../api';
import { openPrivacySettings } from '../privacy';
import { useNoticeStore } from './notices';
import { t } from '../i18n';
import { refreshNameClashes } from './nameClash';
import { useUiStore } from './ui';

/** Nhịp tải lại tối đa khi font đang về (quét/tải cloud): 3 s, đủ thấy thư viện lớn lên mà không dồn lệnh vào DB đang bận ghi. */
export const BUMP_THROTTLE_MS = 3000;
/** `finished` rồi `idle` liền nhau trong khoảng này chỉ tính một lần bump. */
export const IDLE_COALESCE_MS = 500;

export type AccessProblem = 'denied' | 'timeout';
type State = {
  scanning: boolean; written: number; paused: boolean; version: number; access: Record<number, AccessProblem>;
  /** Số file chỉ là placeholder đám mây (chưa tải về) đã bỏ qua trong lần quét đang chạy, theo nguồn. */
  cloud: Record<number, number>;
  cloudTotal(): number;
  /** Tăng khi số liệu NGUỒN (faceCount, cloudOnlyCount…) có thể đã đổi mà thư viện không đổi: quét xong changed=false, tải cloud xong/huỷ. Chỉ nạp lại danh sách nguồn, không nạp lại lưới. */
  sourcesRev: number;
  bumpSources(): void;
  bump(): void;
};
export type ScanStore = UseBoundStore<StoreApi<State>>;

export function createScanStore(): ScanStore {
  return create<State>()((set, get) => ({
    scanning: false, written: 0, paused: false, version: 0, sourcesRev: 0, access: {}, cloud: {},
    cloudTotal: () => Object.values(get().cloud).reduce((a, b) => a + b, 0),
    bumpSources: () => set(s => ({ sourcesRev: s.sourcesRev + 1 })),
    bump: () => set(s => ({ version: s.version + 1 })),
  }));
}
export const useScanStore = createScanStore();

export async function startScanSync(api: Api, store: ScanStore = useScanStore): Promise<() => void> {
  let last = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  /** Lượt hiện tại có đổi gì không: watcher quét không đổi (changed=false) thì không bump/không hiện "đang quét". */
  let dirty = false;
  const quiet = (e: { trigger?: string }) => e.trigger === 'watcher';
  /** Đã bump kể từ lúc nguồn này vào hàng (queued): một lượt quét chỉ refresh một lần. */
  let bumpedThisPass = false;
  const bumpNow = () => { clearTimeout(timer); timer = undefined; last = Date.now(); dirty = false; bumpedThisPass = true; store.getState().bump(); };
  void refreshNameClashes(api);
  // Xoá nguồn, giải quyết trùng…: mọi nơi đổi thư viện đều bump `libraryRev` → nạp lại danh sách trùng tên.
  const offRev = useUiStore.subscribe((s, prev) => { if (s.libraryRev !== prev.libraryRev) void refreshNameClashes(api); });
  const bumpSoon = () => {
    const wait = BUMP_THROTTLE_MS - (Date.now() - last);
    if (wait <= 0) bumpNow(); else if (!timer) timer = setTimeout(bumpNow, wait);
  };
  const off = await api.scanSubscribe(e => {
    const set = store.setState;
    const clear = (id: number) => set(s => { const { [id]: _drop, ...rest } = s.access; return { access: rest }; });
    const dropCloud = (id: number) => set(s => { const { [id]: _d, ...rest } = s.cloud; return { cloud: rest }; });
    if (e.type === 'queued') { bumpedThisPass = false; if (!quiet(e)) set({ scanning: true }); clear(e.sourceId); dropCloud(e.sourceId); }
    else if (e.type === 'progress') set(s => ({ scanning: quiet(e) ? s.scanning : true, written: e.progress.facesWritten, cloud: { ...s.cloud, [e.sourceId]: e.progress.notDownloaded } }));
    else if (e.type === 'facesAdded') { dirty = true; bumpSoon(); }
    else if (e.type === 'paused') set({ paused: true });
    else if (e.type === 'resumed') set({ paused: false });
    else if (e.type === 'finished') {
      clear(e.sourceId); dropCloud(e.sourceId);
      // Thư viện không đổi nhưng còn file chưa tải: cloudOnlyCount của nguồn đã đổi → chỉ nạp lại danh sách nguồn (changed=true đã bump cả version).
      const refreshSourcesOnly = e.changed === false && e.stats.notDownloaded > 0;
      if (refreshSourcesOnly) store.getState().bumpSources();
      if (e.changed !== false || e.stats.purged > 0) {
        // facesAdded đã bump trong lượt này và không còn bump chờ: finished chỉ bump lại khi có đổi mà facesAdded không báo (xoá/đổi chỗ/lỗi).
        const onlyAdds = e.stats.missing + e.stats.purged + e.stats.relinked + e.stats.failed === 0;
        if (timer || !bumpedThisPass || !onlyAdds) bumpNow();
        void refreshNameClashes(api);
      } } // từ đây số chưa tải nằm trong lần quét cuối của nguồn
    else if (e.type === 'failed') {
      set({ scanning: false });
      const d = e.error.detail as { source?: string; op?: string; reason?: string } | null;
      if (d?.source === 'app' && d.op === 'opendir' && (d.reason === 'timeout' || d.reason === 'permission_denied')) {
        // Không đọc được thư mục vì chưa có quyền: font đã quét giữ nguyên, chỉ đánh dấu nguồn cần quyền.
        set(s => ({ access: { ...s.access, [e.sourceId]: d.reason === 'timeout' ? 'timeout' : 'denied' } }));
        void api.listSources().then(list => list.find(x => x.id === e.sourceId)?.name ?? t('scan.thisSource'), () => t('scan.thisSource')).then(name =>
          useNoticeStore.getState().push({
            kind: 'error', text: t('scan.noAccess', { name }),
            action: { label: t('scan.grant'), run: () => { void openPrivacySettings(); } },
          }));
      } else useNoticeStore.getState().push({ kind: 'error', text: t('scan.fail', { code: e.error.code, msg: e.error.message }) });
    } else if (e.type === 'idle') { set({ scanning: false, written: 0 }); if (dirty && Date.now() - last > IDLE_COALESCE_MS) bumpNow(); dirty = false; } // `finished` vừa bump liền trước thì gộp (1 lần nạp, không 2); xác nhận total=0 khi hết quét do LibraryGrid tự làm (scanning true→false)
  }, 'scan.store');
  try {
    const st = await api.scanStatus();
    if (st.running) store.setState({ scanning: true, paused: st.paused });
  } catch (e) { console.warn('scan_status failed', e); }
  return () => { off(); offRev(); clearTimeout(timer); };
}
