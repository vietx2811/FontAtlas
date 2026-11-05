// Mô phỏng model + hàng đợi embed bằng timer (thay Channel của 6a) để dev/QA thấy mọi trạng thái E0–E11.
import type { EmbedEvent, EmbedStatusDto, ModelEvent, ModelStatusDto } from './types';

export const MODEL_BYTES = 371 * 1024 * 1024;
export const EMBED_TOTAL = 3800;

export type ModelSimOptions = {
  tickMs?: number;
  embedEnabled(): boolean;
  initial?: { model?: Partial<ModelStatusDto>; embed?: Partial<EmbedStatusDto> };
};

export function createModelSim(o: ModelSimOptions) {
  const tick = o.tickMs ?? 400;
  let model: ModelStatusDto = {
    state: 'missing', doneBytes: 0, totalBytes: MODEL_BYTES, bytesPerSec: 0, etaMs: null,
    location: '~/Library/Application Support/com.vietx.fontatlas/models', diskFree: 80 * 1024 ** 3, requiredBytes: Math.round(MODEL_BYTES * 1.1), lastError: null,
    ...o.initial?.model,
  };
  let embed: EmbedStatusDto = {
    enabled: false, running: false, paused: false, pending: 0, done: 0, failed: 0, device: null, msPerFace: null, etaMs: null, modelVersion: null, loadError: null,
    ...o.initial?.embed,
  };
  const listeners = new Set<(e: EmbedEvent) => void>();
  let dl: ReturnType<typeof setInterval> | undefined;
  let em: ReturnType<typeof setInterval> | undefined;
  let sink: ((e: ModelEvent) => void) | undefined; // kênh của lần tải đang chạy
  const emit = (e: EmbedEvent) => listeners.forEach(l => l(e));
  const status = () => emit({ kind: 'status', status: { ...embed } });

  const stopEmbedTimer = () => { clearInterval(em); em = undefined; };
  const embedTick = () => {
    if (!embed.running || embed.paused) return;
    const batch = Math.min(embed.pending, 200);
    embed = { ...embed, pending: embed.pending - batch, done: embed.done + batch, etaMs: (embed.pending - batch) * (embed.msPerFace ?? 28) };
    if (embed.pending === 0) {
      embed = { ...embed, running: false, etaMs: null };
      stopEmbedTimer(); status();
      emit({ kind: 'finished', totalDone: embed.done });
    } else status();
  };
  const startEmbed = (pending = EMBED_TOTAL) => {
    embed = { ...embed, enabled: true, running: true, paused: false, pending, done: embed.done, device: 'metal', msPerFace: 28, etaMs: pending * 28, modelVersion: 'siglip2', loadError: null };
    stopEmbedTimer(); em = setInterval(embedTick, tick); status();
  };
  if (embed.running) { em = setInterval(embedTick, tick); }

  const download = (onEvent: (e: ModelEvent) => void) => {
    if (model.state === 'downloading' || model.state === 'ready') return;
    sink = onEvent;
    model = { ...model, state: 'downloading', lastError: null, bytesPerSec: MODEL_BYTES / 10 / (tick / 1000) };
    clearInterval(dl);
    dl = setInterval(() => {
      const done = Math.min(MODEL_BYTES, model.doneBytes + MODEL_BYTES / 10);
      const etaMs = Math.round(((MODEL_BYTES - done) / model.bytesPerSec) * 1000);
      model = { ...model, doneBytes: done, etaMs };
      onEvent({ type: 'progress', done, total: MODEL_BYTES, bytesPerSec: model.bytesPerSec, etaMs, resumed: true });
      if (done >= MODEL_BYTES) {
        clearInterval(dl); dl = undefined;
        model = { ...model, state: 'ready', etaMs: null, bytesPerSec: 0 };
        onEvent({ type: 'ready' });
        if (o.embedEnabled()) startEmbed();
      }
    }, tick);
  };

  return {
    async modelStatus() { return { ...model }; },
    async modelDownload(onEvent: (e: ModelEvent) => void) { download(onEvent); },
    async modelCancel() {
      if (model.state !== 'downloading') return;
      clearInterval(dl); dl = undefined;
      model = { ...model, state: 'paused', bytesPerSec: 0, etaMs: null };
      sink?.({ type: 'paused' });
    },
    async modelRedownload(onEvent: (e: ModelEvent) => void) {
      clearInterval(dl); stopEmbedTimer();
      model = { ...model, state: 'missing', doneBytes: 0, lastError: null };
      embed = { ...embed, running: false, pending: 0 };
      download(onEvent);
    },
    async modelRemove() {
      clearInterval(dl); stopEmbedTimer();
      model = { ...model, state: 'missing', doneBytes: 0, bytesPerSec: 0, etaMs: null, lastError: null };
      embed = { ...embed, running: false, paused: false, pending: 0, done: 0, modelVersion: null };
      status();
    },
    async embedStatus() { return { ...embed }; },
    async embedPause() { if (embed.running) { embed = { ...embed, paused: true }; status(); } },
    async embedResume() { if (embed.running) { embed = { ...embed, paused: false }; status(); } },
    async embedSubscribe(l: (e: EmbedEvent) => void) { listeners.add(l); return () => { listeners.delete(l); }; },
    /** Khi `embedEnabled` đổi: tắt thì dừng hàng đợi; bật mà model đã sẵn sàng thì chạy. */
    embedEnabledChanged(on: boolean) {
      embed = { ...embed, enabled: on };
      if (!on) { embed = { ...embed, running: false, paused: false }; stopEmbedTimer(); status(); }
      else if (model.state === 'ready' && !embed.running && embed.pending === 0 && embed.done === 0) startEmbed();
      else status();
    },
  };
}
