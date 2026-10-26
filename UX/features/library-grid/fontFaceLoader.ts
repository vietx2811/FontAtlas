// Bộ nạp FontFace theo chính sách b2 (spike 05): dedupe, 6 đồng thời, ngân sách theo byte,
// chỉ tải khi tốc độ cuộn < V_MAX hoặc đã ngừng cuộn IDLE_MS. Face ≥ 1 trong TTC không bao giờ dùng FontFace.
import type { LibraryApi } from './api';
import type { FaceRow } from './types';

export const LOADER = { BUDGET_BYTES: 128 * 1024 * 1024, MAX_ACTIVE: 6, V_MAX: 1500, IDLE_MS: 120 } as const;

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

type State = 'waiting' | 'queued' | 'loading' | 'ready' | 'error';
type Entry = {
  state: State; refs: number; used: number; bytes: number; url: string;
  face?: unknown; promise: Promise<string | null>; resolve: (v: string | null) => void;
};

export class FontFaceLoader {
  private entries = new Map<string, Entry>();
  private queue: string[] = [];
  private active = 0;
  private readyBytes = 0;
  private loads = 0;
  private evictions = 0;
  private cancels = 0;
  private idleTimer: unknown = null;
  private disposed = false;
  private opts: { BUDGET_BYTES: number; MAX_ACTIVE: number; V_MAX: number; IDLE_MS: number };

  constructor(private deps: LoaderDeps, opts: Partial<typeof LOADER> = {}) {
    this.opts = { ...LOADER, ...opts };
  }

  acquire(row: LoaderRow): Promise<string | null> {
    const url = this.deps.fontUrl(row);
    if (url === null || this.disposed) return Promise.resolve(null); // gồm faceIndex > 0
    const key = familyName(row);
    const hit = this.entries.get(key);
    if (hit) {
      hit.refs++;
      hit.used = this.deps.now();
      return hit.promise;
    }
    let resolve!: (v: string | null) => void;
    const promise = new Promise<string | null>(r => (resolve = r));
    const e: Entry = { state: 'waiting', refs: 1, used: this.deps.now(), bytes: row.fileSize, url, promise, resolve };
    this.entries.set(key, e);
    if (this.deps.velocity() < this.opts.V_MAX) {
      e.state = 'queued';
      this.queue.push(key);
      this.pump();
    }
    return promise;
  }

  release(row: LoaderRow) {
    const key = familyName(row);
    const e = this.entries.get(key);
    if (!e) return;
    e.refs = Math.max(0, e.refs - 1);
    if (e.refs === 0 && (e.state === 'waiting' || e.state === 'queued')) {
      this.entries.delete(key);
      const i = this.queue.indexOf(key);
      if (i >= 0) this.queue.splice(i, 1);
      this.cancels++;
      e.resolve(null);
    } else if (e.refs === 0 && e.state === 'ready') {
      this.evict(); // vượt ngân sách thì gỡ ngay, không chờ lần tải sau
    }
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
    this.readyBytes = 0;
  }

  onScroll() {
    if (this.idleTimer !== null) this.deps.clearTimer(this.idleTimer);
    this.idleTimer = this.deps.setTimer(() => { this.idleTimer = null; this.flushWaiting(); }, this.opts.IDLE_MS);
    if (this.deps.velocity() < this.opts.V_MAX) this.flushWaiting();
  }

  private flushWaiting() {
    for (const [key, e] of this.entries) {
      if (e.state === 'waiting' && e.refs > 0) { e.state = 'queued'; this.queue.push(key); }
    }
    this.pump();
  }

  private pump() {
    while (this.active < this.opts.MAX_ACTIVE && this.queue.length) {
      const key = this.queue.shift()!;
      const e = this.entries.get(key);
      if (!e || e.state !== 'queued') continue;
      e.state = 'loading';
      this.active++;
      const face = this.deps.createFace(key, `url("${e.url}")`);
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
      ).finally(() => { this.active--; this.pump(); });
    }
  }

  private evict() {
    if (this.readyBytes <= this.opts.BUDGET_BYTES) return;
    const idle = [...this.entries.entries()].filter(([, e]) => e.state === 'ready' && e.refs === 0).sort((a, b) => a[1].used - b[1].used);
    for (const [key, e] of idle) {
      if (this.readyBytes <= this.opts.BUDGET_BYTES) break;
      this.deps.fonts.delete(e.face);
      this.readyBytes -= e.bytes;
      this.entries.delete(key);
      this.evictions++;
    }
  }

  stats() {
    return { active: this.active, readyBytes: this.readyBytes, loads: this.loads, evictions: this.evictions, cancels: this.cancels };
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
