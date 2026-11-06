// Trạng thái model + phân tích hình ảnh (UX 03 §3, E0–E11). Hàm thuần deriveModelUi + store nối với Api (Channel giả ở mock).
import { useMemo } from 'react';
import { create, type StoreApi, type UseBoundStore } from 'zustand';
import type { Api, EmbedStatusDto, ModelEvent, ModelStatusDto, ErrorInfoDto } from '../api';
import { useNoticeStore } from './notices';

export type EState = 'E0' | 'E1' | 'E2' | 'E3' | 'E4' | 'E5' | 'E6' | 'E7' | 'E8' | 'E9' | 'E10' | 'E11';
export type ModelAction = 'download' | 'stop' | 'retry' | 'redownload' | 'remove' | 'pause' | 'resume';
export type ModelUi = {
  code: EState;
  /** Dòng ở chân sidebar; null = không hiện. */
  text: string | null;
  detail?: string;
  checking?: boolean;
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

/** ErrorInfo.message là chi tiết kỹ thuật, không dùng làm thông báo chính. */
export function modelErrorText(error: ErrorInfoDto): string {
  if (error.code === 'http' && /\b429\b/.test(error.message)) {
    const match = /Retry-After\s*[:=]\s*(?:Some\(["']?([^"')]+)["']?\)|["']?([^;\n"']+))/i.exec(error.message);
    const retry = (match?.[1] ?? match?.[2] ?? '').trim();
    if (/^\d+$/.test(retry)) return `Máy chủ đang giới hạn yêu cầu. Thử lại sau ${retry} giây.`;
    const at = Date.parse(retry);
    if (Number.isFinite(at)) return `Máy chủ đang giới hạn yêu cầu. Thử lại lúc ${new Intl.DateTimeFormat('vi-VN', { hour: '2-digit', minute: '2-digit', day: '2-digit', month: '2-digit' }).format(at)}.`;
    return 'Máy chủ đang giới hạn yêu cầu. Hãy thử lại sau.';
  }
  const messages: Record<string, string> = {
    http: 'Máy chủ tải model trả lỗi. Hãy thử lại sau.',
    tls: 'Không xác thực được máy chủ tải. Kiểm tra mạng hoặc chứng chỉ proxy.',
    http_redirect: 'Địa chỉ tải chuyển hướng không hợp lệ.',
    http_config: 'Cấu hình tải model không hợp lệ.',
    network: 'Kết nối tải model bị gián đoạn.',
    io: 'Không đọc/ghi được tệp model. Kiểm tra ổ đĩa và quyền truy cập.',
    checksum: 'Model tải về bị hỏng',
    cancelled: 'Đã dừng tải model',
  };
  return messages[error.code] ?? 'Không tải được model. Hãy thử lại.';
}

export function explainModelError(e: unknown): string {
  const err = typeof e === 'object' && e !== null ? e as { code?: string; message?: string; detail?: { required?: number; free?: number } } : {};
  if (err.code === 'busy') return 'Model đang tải: hãy dừng lần tải hiện tại trước khi tải lại.';
  if (err.code === 'insufficient_disk' && err.detail?.required != null) {
    return `Không đủ chỗ trống: cần ${Math.round(err.detail.required / (1024 * 1024))} MB, còn ${Math.round((err.detail.free ?? 0) / (1024 * 1024))} MB.`;
  }
  return modelErrorText({ code: err.code ?? 'unknown', message: err.message ?? '' });
}
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
  retryDl: { id: 'download', label: 'Thử lại' }, retry: { id: 'retry', label: 'Thử lại' }, redownload: { id: 'redownload', label: 'Thử lại' },
  remove: { id: 'remove', label: 'Xoá model' }, pause: { id: 'pause', label: 'Tạm dừng' }, resume: { id: 'resume', label: 'Tiếp tục' },
} as const satisfies Record<string, { id: ModelAction; label: string }>;

export function deriveModelUi(i: ModelUiInput): ModelUi {
  const base = { severity: 'info' as const, actions: [] as ModelUi['actions'], progress: null, imageReady: false };
  const { model: m, embed: e } = i;
  if (!m) return { ...base, code: 'E0', text: null };
  const transferState = ['downloading', 'paused', 'checksum_mismatch', 'corrupt'].includes(m.state) || !!m.lastError;
  if (!i.embedEnabled && !transferState) return { ...base, code: 'E0', text: null };
  const err = m.lastError?.code;
  if (m.state === 'downloading') {
    if (m.totalBytes > 0 && m.doneBytes >= m.totalBytes) return { ...base, code: 'E2', text: 'Đang kiểm tra tệp…', checking: true, actions: [A.stop] };
    const ratio = m.totalBytes > 0 ? Math.max(0, m.doneBytes / m.totalBytes) : null;
    const pct = ratio === null ? null : Math.min(99, Math.round(ratio * 100));
    const speed = m.bytesPerSec > 0 && Number.isFinite(m.bytesPerSec) ? `${new Intl.NumberFormat('vi-VN', { maximumFractionDigits: 1 }).format(m.bytesPerSec / 1_000_000)} MB/s` : '';
    const text = [pct === null ? 'Đang tải model…' : `Đang tải model ${pct}%`, m.totalBytes > 0 ? `${Math.round(m.doneBytes / MB)}/${Math.round(m.totalBytes / MB)} MB` : '', speed, formatEta(m.etaMs)].filter(Boolean).join(' · ');
    return { ...base, code: 'E2', text, actions: [A.stop], progress: ratio };
  }
  if (err === 'cancelled') return { ...base, code: 'E3', text: modelErrorText(m.lastError!), detail: m.lastError!.message, actions: [A.resumeDl] };
  if (m.state === 'checksum_mismatch' || err === 'checksum') return { ...base, code: 'E5', text: modelErrorText({ code: 'checksum', message: '' }), severity: 'error', actions: [A.redownload], detail: m.lastError?.message };
  if (m.lastError) return { ...base, code: 'E4', text: modelErrorText(m.lastError), severity: 'error', actions: [A.retryDl], detail: m.lastError.message };
  if (m.state === 'paused') return { ...base, code: 'E3', text: 'Đã dừng tải model', actions: [A.resumeDl], progress: m.totalBytes > 0 ? m.doneBytes / m.totalBytes : null };
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
    let revision = 0;
    const refresh = async (api: Api) => {
      const before = revision;
      const [model, embed] = await Promise.all([api.modelStatus(), api.embedStatus()]);
      set(before === revision ? { model, embed } : { embed });
    };
    const onModel = (api: Api) => (e: ModelEvent) => {
      revision++;
      if (e.type === 'progress') {
        set(s => s.model ? { model: { ...s.model, state: 'downloading', doneBytes: e.done, totalBytes: e.total, bytesPerSec: e.bytesPerSec, etaMs: e.etaMs, lastError: null } } : {});
      } else {
        set(s => s.model ? { model: { ...s.model, state: e.type === 'ready' ? 'ready' : e.type === 'failed' && e.error.code === 'checksum' ? 'checksum_mismatch' : 'paused', bytesPerSec: 0, etaMs: null, lastError: e.type === 'failed' ? e.error : null } } : {});
        void refresh(api);
      }
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
        try {
          if ((await api.modelStatus()).state !== 'ready') await api.modelDownload(onModel(api));
        } catch (e) { useNoticeStore.getState().push({ kind: (e as { code?: string })?.code === 'cancelled' ? 'info' : 'error', text: explainModelError(e) }); }
        await refresh(api);
      },
      async act(api, id) {
        try { await runAct(id); } catch (e) { useNoticeStore.getState().push({ kind: (e as { code?: string })?.code === 'cancelled' ? 'info' : 'error', text: explainModelError(e) }); }
        await refresh(api);
        async function runAct(id: ModelAction) {
        if (id === 'download') await api.modelDownload(onModel(api));
        else if (id === 'stop') await api.modelCancel();
        else if (id === 'redownload') await api.modelRedownload(onModel(api));
        else if (id === 'remove') await api.modelRemove();
        else if (id === 'pause') await api.embedPause();
        else if (id === 'resume') await api.embedResume();
        else if (id === 'retry') await api.embedResume();
        }
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
