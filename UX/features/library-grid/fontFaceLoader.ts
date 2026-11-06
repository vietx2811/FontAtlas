// Bộ nạp FontFace theo chính sách b2 (spike 05): dedupe, 6 đồng thời, ngân sách theo byte,
// chỉ tải khi tốc độ cuộn < V_MAX hoặc đã ngừng cuộn IDLE_MS. Face ≥ 1 trong TTC không bao giờ dùng FontFace.
import type { LibraryApi } from './api';
import type { FaceRow } from './types';

export const LOADER = {
  BUDGET_BYTES: 128 * 1024 * 1024, MAX_ACTIVE: 6, V_MAX: 1500, IDLE_MS: 120,
  /** Làn "tên": tối đa 2 lượt tải đồng thời, không tính vào MAX_ACTIVE nên không bao giờ chặn slot của hover/live/panel. */
  NAME_MAX_ACTIVE: 2,
  /** Font lớn hơn mức này (CJK, font màu…) không được nạp chỉ để vẽ tên: giữ font giao diện, Tooltip hiện tên đầy đủ. */
  NAME_MAX_BYTES: 6 * 1024 * 1024,
} as const;

export class ScrollVelocity {
  private v = 0;
  private lastTop = 0;
  private lastT: number | null = null;
  /** EWMA như spike (0,7 cũ / 0,3 mới). */
  sample(top: number, t: number) {
    if (this.lastT !== null) {
      const dt = Math.max(1, t - this.lastT);
      this.v = 0.7 * this.v + 0.3 * Math.abs(top - this.lastTop) / dt * 1000;
    }
    this.lastTop = top;
    this.lastT = t;
  }
  value(t: number) {
    return this.lastT === null || t - this.lastT > LOADER.IDLE_MS ? 0 : this.v;
  }
}

export type LoaderRow = Pick<FaceRow, 'hashHex' | 'faceIndex' | 'fileSize'>;
export type LoaderDeps = {
  createFace(family: string, source: string): { load(): Promise<unknown> };
  fonts: { add(f: unknown): void; delete(f: unknown): boolean };
  fontUrl(row: Pick<FaceRow, 'hashHex' | 'faceIndex'>): string | null;
  now(): number;
  velocity(): number;
  setTimer(fn: () => void, ms: number): unknown;
  clearTimer(h: unknown): void;
};

export function familyName(row: Pick<FaceRow, 'hashHex' | 'faceIndex'>): string {
  // Đủ 64 ký tự hash: tiền tố ngắn có thể trùng giữa hai font khác nhau.
  return `fa-${row.hashHex}-${row.faceIndex}`;
}

type State = 'waiting' | 'queued' | 'loading' | 'ready' | 'error' | 'evicted';
type Entry = {
  state: State;
  /** Ref cứng: hover/live, nội dung panel. Ưu tiên cao, không bị gỡ khi vượt ngân sách. */
  refs: number;
  /** Ref mềm: chỉ để vẽ tên. Làn ưu tiên thấp; khi vượt ngân sách font chỉ còn ref mềm được phép gỡ (tên rơi về font giao diện). */
  soft: number;
  used: number; bytes: number; url: string;
  /** Đang xếp/đang tải ở làn tên. */
  low: boolean;
  face?: unknown; promise: Promise<string | null>; resolve: (v: string | null) => void;
};
export type AcquireOpts = { soft?: boolean };

export class FontFaceLoader {
  private entries = new Map<string, Entry>();
  private queue: string[] = [];
  private lowQueue: string[] = [];
  private active = 0;
  private lowActive = 0;
  private readyBytes = 0;
  private loads = 0;
  private evictions = 0;
  private cancels = 0;
  private scrolling = false;
  private idleTimer: unknown = null;
  private disposed = false;
  private opts: { BUDGET_BYTES: number; MAX_ACTIVE: number; V_MAX: number; IDLE_MS: number; NAME_MAX_ACTIVE: number; NAME_MAX_BYTES: number };

  constructor(private deps: LoaderDeps, opts: Partial<typeof LOADER> = {}) {
    this.opts = { ...LOADER, ...opts };
  }

  /** `soft`: chỉ để vẽ tên (làn thấp). Mặc định là ref cứng. */
  acquire(row: LoaderRow, o: AcquireOpts = {}): Promise<string | null> {
    const soft = !!o.soft;
    const url = this.deps.fontUrl(row);
    if (url === null || this.disposed) return Promise.resolve(null); // gồm faceIndex > 0
    const key = familyName(row);
    const hit = this.entries.get(key);
    if (hit) {
      if (soft) hit.soft++; else hit.refs++;
      hit.used = this.deps.now();
      if (hit.state === 'evicted') this.revive(key, hit);
      else if (!soft && hit.low && (hit.state === 'queued' || hit.state === 'waiting')) this.promote(key, hit);
      return hit.promise;
    }
    if (soft && row.fileSize > this.opts.NAME_MAX_BYTES) return Promise.resolve(null);
    let resolve!: (v: string | null) => void;
    const promise = new Promise<string | null>(r => (resolve = r));
    const e: Entry = { state: 'waiting', refs: soft ? 0 : 1, soft: soft ? 1 : 0, used: this.deps.now(), bytes: row.fileSize, url, low: soft, promise, resolve };
    this.entries.set(key, e);
    this.admit(key, e);
    return promise;
  }

  release(row: LoaderRow, o: AcquireOpts = {}) {
    const key = familyName(row);
    const e = this.entries.get(key);
    if (!e) return;
    if (o.soft) e.soft = Math.max(0, e.soft - 1); else e.refs = Math.max(0, e.refs - 1);
    if (e.refs + e.soft === 0) {
      if (e.state === 'waiting' || e.state === 'queued') {
        this.entries.delete(key);
        this.dequeue(key);
        this.cancels++;
        e.resolve(null);
      } else if (e.state === 'evicted') {
        this.entries.delete(key);
      } else if (e.state === 'ready') this.evict(); // vượt ngân sách thì gỡ ngay, không chờ lần tải sau
    } else if (e.refs === 0 && e.state === 'ready') this.evict();
  }

  /** Gỡ mọi FontFace khỏi document.fonts, huỷ việc đang chờ; sau đó acquire luôn trả null. */
  dispose() {
    this.disposed = true;
    if (this.idleTimer !== null) { this.deps.clearTimer(this.idleTimer); this.idleTimer = null; }
    for (const e of this.entries.values()) {
      if (e.state === 'ready') this.deps.fonts.delete(e.face);
      else e.resolve(null);
    }
    this.entries.clear();
    this.queue = [];
    this.lowQueue = [];
    this.readyBytes = 0;
  }

  onScroll() {
    this.scrolling = true;
    if (this.idleTimer !== null) this.deps.clearTimer(this.idleTimer);
    this.idleTimer = this.deps.setTimer(() => { this.idleTimer = null; this.scrolling = false; this.flushWaiting(true); }, this.opts.IDLE_MS);
    if (this.deps.velocity() < this.opts.V_MAX) this.flushWaiting(false);
  }

  /** Xếp hàng theo làn (hoặc chờ nếu đang cuộn nhanh / đang cuộn với làn tên). */
  private admit(key: string, e: Entry) {
    if (e.low ? !this.scrolling : this.deps.velocity() < this.opts.V_MAX) {
      e.state = 'queued';
      (e.low ? this.lowQueue : this.queue).push(key);
      this.pump();
    } else e.state = 'waiting';
  }

  private dequeue(key: string) {
    for (const q of [this.queue, this.lowQueue]) { const i = q.indexOf(key); if (i >= 0) q.splice(i, 1); }
  }

  /** Hover/live/panel đòi font đang chờ ở làn tên → chuyển lên làn cao. */
  private promote(key: string, e: Entry) {
    this.dequeue(key);
    e.low = false;
    e.state = 'waiting';
    this.admit(key, e);
  }

  /** Font đã bị gỡ vì vượt ngân sách mà lại có người cần: nạp lại. */
  private revive(key: string, e: Entry) {
    e.promise = new Promise<string | null>(r => (e.resolve = r));
    e.low = e.refs === 0;
    e.state = 'waiting';
    this.admit(key, e);
  }

  private flushWaiting(includeLow: boolean) {
    for (const [key, e] of this.entries) {
      if (e.state !== 'waiting' || e.refs + e.soft === 0) continue;
      if (e.low && !includeLow) continue;
      e.state = 'queued';
      (e.low ? this.lowQueue : this.queue).push(key);
    }
    this.pump();
  }

  private pump() {
    while (this.active < this.opts.MAX_ACTIVE && this.queue.length) this.start(this.queue.shift()!, false);
    if (!this.scrolling) while (this.lowActive < this.opts.NAME_MAX_ACTIVE && this.lowQueue.length) this.start(this.lowQueue.shift()!, true);
  }

  private start(key: string, low: boolean) {
    const e = this.entries.get(key);
    if (!e || e.state !== 'queued') return;
    e.state = 'loading';
    if (low) this.lowActive++; else this.active++;
    let face: { load(): Promise<unknown> };
    try { face = this.deps.createFace(key, `url("${e.url}")`); }
    catch { e.state = 'error'; e.resolve(null); if (low) this.lowActive--; else this.active--; return; } // môi trường không có FontFace: giữ font giao diện
    face.load().then(
      () => {
        if (this.disposed) { e.resolve(null); return; }
        this.deps.fonts.add(face);
        e.face = face;
        e.state = 'ready';
        this.readyBytes += e.bytes;
        this.loads++;
        e.resolve(key);
        this.evict();
      },
      () => { e.state = 'error'; e.resolve(null); },
    ).finally(() => { if (low) this.lowActive--; else this.active--; this.pump(); });
  }

  /** Vượt ngân sách: gỡ font không ai giữ (LRU) trước; chưa đủ thì gỡ font chỉ còn ref mềm (tên) — ref cứng không bao giờ bị gỡ. */
  private evict() {
    if (this.readyBytes <= this.opts.BUDGET_BYTES) return;
    const ready = [...this.entries.entries()].filter(([, e]) => e.state === 'ready');
    const byUse = (a: [string, Entry], b: [string, Entry]) => a[1].used - b[1].used;
    const idle = ready.filter(([, e]) => e.refs + e.soft === 0).sort(byUse);
    const softOnly = ready.filter(([, e]) => e.refs === 0 && e.soft > 0).sort(byUse);
    for (const [key, e] of [...idle, ...softOnly]) {
      if (this.readyBytes <= this.opts.BUDGET_BYTES) break;
      this.deps.fonts.delete(e.face);
      this.readyBytes -= e.bytes;
      this.evictions++;
      // Còn ref mềm: giữ mục để release sau này cân bằng; tên tự rơi về font giao diện khi FontFace bị gỡ.
      if (e.refs + e.soft === 0) this.entries.delete(key); else e.state = 'evicted';
    }
  }

  stats() {
    let refs = 0; let soft = 0;
    for (const e of this.entries.values()) { refs += e.refs; soft += e.soft; }
    return { active: this.active, lowActive: this.lowActive, readyBytes: this.readyBytes, loads: this.loads, evictions: this.evictions, cancels: this.cancels, refs, soft, entries: this.entries.size };
  }
}

export function browserLoaderDeps(api: LibraryApi, velocity: ScrollVelocity): LoaderDeps {
  return {
    createFace: (f, src) => new FontFace(f, src),
    fonts: { add: f => void document.fonts.add(f as FontFace), delete: f => document.fonts.delete(f as FontFace) },
    fontUrl: r => api.fontUrl(r),
    now: () => performance.now(),
    velocity: () => velocity.value(performance.now()),
    setTimer: (fn, ms) => setTimeout(fn, ms),
    clearTimer: h => clearTimeout(h as ReturnType<typeof setTimeout>),
  };
}
