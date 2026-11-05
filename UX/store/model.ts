// Trạng thái model + phân tích hình ảnh (UX 03 §3, E0–E11). Hàm thuần deriveModelUi + store nối với Api (Channel giả ở mock).
import { useMemo } from 'react';
import { create, type StoreApi, type UseBoundStore } from 'zustand';
import type { Api, EmbedStatusDto, ModelEvent, ModelStatusDto } from '../api';

export type EState = 'E0' | 'E1' | 'E2' | 'E3' | 'E4' | 'E5' | 'E6' | 'E7' | 'E8' | 'E9' | 'E10' | 'E11';
export type ModelAction = 'download' | 'stop' | 'retry' | 'redownload' | 'remove' | 'pause' | 'resume';
export type ModelUi = {
  code: EState;
  /** Dòng ở chân sidebar; null = không hiện. */
  text: string | null;
  detail?: string;
  severity: 'info' | 'error';
  actions: Array<{ id: ModelAction; label: string }>;
  /** 0..1 cho vòng tiến độ. */
  progress: number | null;
  /** Dòng cho Cài đặt (E11). */
  settingsText?: string;
  /** Model sẵn sàng và phân tích bật: Similar/Pairing có thể dùng hình ảnh. */
  imageReady: boolean;
};
export type ModelUiInput = { embedEnabled: boolean; justFinished: boolean; e1Dismissed: boolean; model: ModelStatusDto | null; embed: EmbedStatusDto | null };

export const fmtInt = (n: number): string => String(Math.round(n)).replace(/\B(?=(\d{3})+(?!\d))/g, '.');
export function formatEta(ms: number | null): string {
  if (ms === null) return '';
  if (ms < 60_000) return 'dưới 1 phút';
  if (ms < 3600_000) return `~${Math.round(ms / 60_000)} phút`;
  return `~${Math.round(ms / 3600_000)} giờ`;
}
const MB = 1024 * 1024;
const A = {
  download: { id: 'download', label: 'Tải' }, resumeDl: { id: 'download', label: 'Tiếp tục tải' }, stop: { id: 'stop', label: 'Dừng' },
  retryDl: { id: 'download', label: 'Thử lại' }, retry: { id: 'retry', label: 'Thử lại' }, redownload: { id: 'redownload', label: 'Tải lại' },
  remove: { id: 'remove', label: 'Xoá model' }, pause: { id: 'pause', label: 'Tạm dừng' }, resume: { id: 'resume', label: 'Tiếp tục' },
} as const satisfies Record<string, { id: ModelAction; label: string }>;

export function deriveModelUi(i: ModelUiInput): ModelUi {
  const base = { severity: 'info' as const, actions: [] as ModelUi['actions'], progress: null, imageReady: false };
  const { model: m, embed: e } = i;
  if (!i.embedEnabled || !m) return { ...base, code: 'E0', text: null };
  const err = m.lastError?.code;
  if (m.state === 'downloading') {
    const pct = Math.round((m.doneBytes / m.totalBytes) * 100);
    return { ...base, code: 'E2', text: `Đang tải model ${pct}% · ${Math.round(m.doneBytes / MB)}/${Math.round(m.totalBytes / MB)} MB · ${formatEta(m.etaMs)}`.replace(/ · $/, ''), actions: [A.stop], progress: m.doneBytes / m.totalBytes };
  }
  if ((m.state === 'missing' || m.state === 'paused') && (err === 'network' || err === 'io')) {
    return { ...base, code: 'E4', text: 'Tải model lỗi mạng', severity: 'error', actions: [A.retryDl], detail: m.lastError!.message };
  }
  if (m.state === 'paused') return { ...base, code: 'E3', text: 'Đã dừng tải model', actions: [A.resumeDl], progress: m.doneBytes / m.totalBytes };
  if (m.state === 'checksum_mismatch') return { ...base, code: 'E5', text: 'Model tải về bị hỏng', severity: 'error', actions: [A.redownload], detail: 'File tải về không khớp checksum, đã xoá. Có thể do mạng hoặc proxy.' };
  if (m.state === 'corrupt') return { ...base, code: 'E6', text: 'Model hình ảnh bị hỏng', severity: 'error', actions: [A.redownload, A.remove] };
  if (m.state === 'missing') return { ...base, code: 'E1', text: i.e1Dismissed ? null : 'Chưa tải model hình ảnh', actions: [A.download] };
  if (m.state !== 'ready') return { ...base, code: 'E0', text: null };

  const ready = { ...base, imageReady: true };
  if (e?.loadError) return { ...ready, code: 'E7', text: 'Không chạy được phân tích hình ảnh', severity: 'error', actions: [A.retry], detail: e.loadError };
  if (e && e.running) {
    const total = e.done + e.pending + e.failed;
    if (e.paused) return { ...ready, code: 'E9', text: `Phân tích hình ảnh tạm dừng ${fmtInt(e.done)}/${fmtInt(total)}`, actions: [A.resume], progress: e.done / total };
    return {
      ...ready, code: 'E8', text: `Phân tích hình ảnh ${fmtInt(e.done)}/${fmtInt(total)} · ${formatEta(e.etaMs)}`.replace(/ · $/, ''), actions: [A.pause], progress: e.done / total,
      detail: e.device ? `Đang dùng: ${e.device === 'metal' ? 'Metal (GPU)' : 'CPU'}` : undefined,
    };
  }
  if (i.justFinished && e) return { ...ready, code: 'E10', text: `Đã phân tích hình ảnh ${fmtInt(e.done)} font` };
  if (e && e.failed > 0) return { ...ready, code: 'E11', text: null, settingsText: `${fmtInt(e.failed)} font không phân tích được hình ảnh` };
  return { ...ready, code: 'E0', text: null };
}

type State = {
  embedEnabled: boolean; model: ModelStatusDto | null; embed: EmbedStatusDto | null; justFinished: boolean; e1Dismissed: boolean;
  ui(): ModelUi;
  sync(api: Api): Promise<() => void>;
  enable(api: Api): Promise<void>;
  act(api: Api, id: ModelAction): Promise<void>;
  dismissE1(): void;
  clearFinished(): void;
};
export type ModelStore = UseBoundStore<StoreApi<State>>;

export function createModelStore(): ModelStore {
  return create<State>()((set, get) => {
    const refresh = async (api: Api) => set({ model: await api.modelStatus(), embed: await api.embedStatus() });
    const onModel = (api: Api) => (e: ModelEvent) => {
      if (e.type === 'progress') {
        set(s => s.model && { model: { ...s.model, state: 'downloading', doneBytes: e.done, totalBytes: e.total, bytesPerSec: e.bytesPerSec, etaMs: e.etaMs } });
      } else void refresh(api);
    };
    return {
      embedEnabled: false, model: null, embed: null, justFinished: false, e1Dismissed: false,
      ui: () => deriveModelUi(get()),
      async sync(api) {
        const settings = await api.getSettings();
        set({ embedEnabled: settings.embedEnabled });
        await refresh(api);
        let timer: ReturnType<typeof setTimeout> | undefined;
        const off = await api.embedSubscribe(e => {
          if (e.kind === 'status') set({ embed: e.status });
          else { set({ justFinished: true }); clearTimeout(timer); timer = setTimeout(() => get().clearFinished(), 5000); }
        });
        return () => { off(); clearTimeout(timer); };
      },
      async enable(api) {
        await api.setSettings({ embedEnabled: true });
        set({ embedEnabled: true, e1Dismissed: false });
        if ((await api.modelStatus()).state !== 'ready') { await api.modelDownload(onModel(api)); await refresh(api); }
        else await refresh(api);
      },
      async act(api, id) {
        if (id === 'download') await api.modelDownload(onModel(api));
        else if (id === 'stop') await api.modelCancel();
        else if (id === 'redownload') await api.modelRedownload(onModel(api));
        else if (id === 'remove') await api.modelRemove();
        else if (id === 'pause') await api.embedPause();
        else if (id === 'resume') await api.embedResume();
        else if (id === 'retry') await api.embedResume();
        await refresh(api);
      },
      dismissE1: () => set({ e1Dismissed: true }),
      clearFinished: () => set({ justFinished: false }),
    };
  });
}

export const useModelStore = createModelStore();

/** Hook chọn từng trường nguyên thuỷ rồi tính ui bằng useMemo (selector trả object mới sẽ lặp render). */
export function useModelUi(store: ModelStore = useModelStore): ModelUi {
  const embedEnabled = store(s => s.embedEnabled);
  const justFinished = store(s => s.justFinished);
  const e1Dismissed = store(s => s.e1Dismissed);
  const model = store(s => s.model);
  const embed = store(s => s.embed);
  return useMemo(() => deriveModelUi({ embedEnabled, justFinished, e1Dismissed, model, embed }), [embedEnabled, justFinished, e1Dismissed, model, embed]);
}
