// Trạng thái model + phân tích hình ảnh (UX 03 §3, E0–E11). Hàm thuần deriveModelUi + store nối với Api (Channel giả ở mock).
import { useMemo } from 'react';
import { create, type StoreApi, type UseBoundStore } from 'zustand';
import type { Api, EmbedStatusDto, ModelEvent, ModelStatusDto, ErrorInfoDto } from '../api';
import { useNoticeStore } from './notices';
import { t, formatNumber, formatDate, localeTag, useLocale, tn } from '../i18n';

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
    if (/^\d+$/.test(retry)) return t('mdl.rate.sec', { n: retry });
    const at = Date.parse(retry);
    if (Number.isFinite(at)) return t('mdl.rate.at', { at: formatDate(at, { hour: '2-digit', minute: '2-digit', day: '2-digit', month: '2-digit' }) });
    return t('mdl.rate.later');
  }
  const messages: Record<string, string> = {
    http: t('mdl.err.http'),
    tls: t('mdl.err.tls'),
    http_redirect: t('mdl.err.http_redirect'),
    http_config: t('mdl.err.http_config'),
    network: t('mdl.err.network'),
    io: t('mdl.err.io'),
    checksum: t('mdl.err.checksum'),
    model_file: t('mdl.err.model_file'),
    resources: t('mdl.err.resources'),
    inference: t('mdl.err.inference'),
    store: t('mdl.err.store'),
    cancelled: t('mdl.err.cancelled'),
  };
  return messages[error.code] ?? t('mdl.err.generic');
}

export function explainModelError(e: unknown): string {
  const err = typeof e === 'object' && e !== null ? e as { code?: string; message?: string; detail?: { required?: number; free?: number } } : {};
  if (err.code === 'busy') return t('mdl.err.busy');
  if (err.code === 'insufficient_disk' && err.detail?.required != null) {
    return t('mdl.err.disk', { need: Math.round(err.detail.required / (1024 * 1024)), free: Math.round((err.detail.free ?? 0) / (1024 * 1024)) });
  }
  return modelErrorText({ code: err.code ?? 'unknown', message: err.message ?? '' });
}
export const fmtInt = (n: number): string => formatNumber(Math.round(n));
export function formatEta(ms: number | null): string {
  if (ms === null) return '';
  if (ms < 60_000) return t('mdl.eta.lt1');
  if (ms < 3600_000) return t('mdl.eta.min', { n: Math.round(ms / 60_000) });
  return t('mdl.eta.hour', { n: Math.round(ms / 3600_000) });
}
const MB = 1024 * 1024;
// Nhãn đọc lúc dùng (getter) để đổi ngôn ngữ có hiệu lực ngay.
const act = <I extends ModelAction>(id: I, key: Parameters<typeof t>[0]) => ({ id, get label(): string { return t(key); } });
const A = {
  download: act('download', 'mdl.a.download'), resumeDl: act('download', 'mdl.a.resumeDl'), stop: act('stop', 'mdl.a.stop'),
  retryDl: act('download', 'mdl.a.retry'), retry: act('retry', 'mdl.a.retry'), redownload: act('redownload', 'mdl.a.redownload'),
  remove: act('remove', 'mdl.a.remove'), pause: act('pause', 'mdl.a.pause'), resume: act('resume', 'mdl.a.resume'),
} as const satisfies Record<string, { id: ModelAction; label: string }>;

export function deriveModelUi(i: ModelUiInput): ModelUi {
  const base = { severity: 'info' as const, actions: [] as ModelUi['actions'], progress: null, imageReady: false };
  const { model: m, embed: e } = i;
  if (!m) return { ...base, code: 'E0', text: null };
  const transferState = ['downloading', 'paused', 'checksum_mismatch', 'corrupt'].includes(m.state) || !!m.lastError;
  if (!i.embedEnabled && !transferState) return { ...base, code: 'E0', text: null };
  const err = m.lastError?.code;
  if (m.state === 'downloading') {
    if (m.totalBytes > 0 && m.doneBytes >= m.totalBytes) return { ...base, code: 'E2', text: t('mdl.checking'), checking: true, actions: [A.stop] };
    const ratio = m.totalBytes > 0 ? Math.max(0, m.doneBytes / m.totalBytes) : null;
    const pct = ratio === null ? null : Math.min(99, Math.round(ratio * 100));
    const speed = m.bytesPerSec > 0 && Number.isFinite(m.bytesPerSec) ? `${new Intl.NumberFormat(localeTag(), { maximumFractionDigits: 1 }).format(m.bytesPerSec / 1_000_000)} MB/s` : '';
    const text = [pct === null ? t('mdl.downloading') : t('mdl.downloadingPct', { pct }), m.totalBytes > 0 ? `${Math.round(m.doneBytes / MB)}/${Math.round(m.totalBytes / MB)} MB` : '', speed, formatEta(m.etaMs)].filter(Boolean).join(' · ');
    return { ...base, code: 'E2', text, actions: [A.stop], progress: ratio };
  }
  if (err === 'cancelled') return { ...base, code: 'E3', text: modelErrorText(m.lastError!), detail: m.lastError!.message, actions: [A.resumeDl] };
  if (m.state === 'checksum_mismatch' || err === 'checksum' || err === 'model_file') return { ...base, code: 'E5', text: modelErrorText({ code: err === 'model_file' ? 'model_file' : 'checksum', message: '' }), severity: 'error', actions: [A.redownload], detail: m.lastError?.message };
  if (m.lastError) return { ...base, code: 'E4', text: modelErrorText(m.lastError), severity: 'error', actions: [A.retryDl], detail: m.lastError.message };
  if (m.state === 'paused') return { ...base, code: 'E3', text: t('mdl.paused'), actions: [A.resumeDl], progress: m.totalBytes > 0 ? m.doneBytes / m.totalBytes : null };
  if (m.state === 'corrupt') return { ...base, code: 'E6', text: t('mdl.corrupt'), severity: 'error', actions: [A.redownload, A.remove] };
  if (m.state === 'missing') return { ...base, code: 'E1', text: i.e1Dismissed ? null : t('mdl.missing'), actions: [A.download] };
  if (m.state !== 'ready') return { ...base, code: 'E0', text: null };

  const ready = { ...base, imageReady: true };
  // Lỗi nạp/chạy model (mã từ 04): mỗi nhóm một câu "chuyện gì" + "làm gì" và đúng nút. disabled/cancelled không phải lỗi.
  if (e?.loadError && e.loadError !== 'disabled' && e.loadError !== 'cancelled') {
    const code = e.loadError;
    if (code === 'model_file' || code === 'checksum') return { ...ready, code: 'E7', text: modelErrorText({ code, message: '' }), severity: 'error', actions: [A.redownload], detail: code };
    if (code === 'resources' || code === 'inference' || code === 'store' || code === 'io') return { ...ready, code: 'E7', text: modelErrorText({ code, message: '' }), severity: 'error', actions: [A.retry], detail: code };
    return { ...ready, code: 'E7', text: t('mdl.loadError'), severity: 'error', actions: [A.retry], detail: code };
  }
  if (e && e.running) {
    const total = e.done + e.pending + e.failed;
    if (e.paused) return { ...ready, code: 'E9', text: t('mdl.embPaused', { done: fmtInt(e.done), total: fmtInt(total) }), actions: [A.resume], progress: e.done / total };
    return {
      ...ready, code: 'E8', text: t('mdl.emb', { done: fmtInt(e.done), total: fmtInt(total), eta: formatEta(e.etaMs) }).replace(/ · $/, ''), actions: [A.pause], progress: e.done / total,
      detail: e.device ? t('mdl.embDevice', { device: e.device === 'metal' ? 'Metal (GPU)' : 'CPU' }) : undefined,
    };
  }
  if (i.justFinished && e) return { ...ready, code: 'E10', text: tn('mdl.embDone', { n: fmtInt(e.done) }) };
  if (e && e.failed > 0) return { ...ready, code: 'E11', text: null, settingsText: tn('mdl.embFailed', { n: fmtInt(e.failed) }) };
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
  const locale = useLocale(); // nhãn/câu do deriveModelUi dựng bằng t(): tính lại khi đổi ngôn ngữ
  return useMemo(() => deriveModelUi({ embedEnabled, justFinished, e1Dismissed, model, embed }), [embedEnabled, justFinished, e1Dismissed, model, embed, locale]); // eslint-disable-line react-hooks/exhaustive-deps
}
