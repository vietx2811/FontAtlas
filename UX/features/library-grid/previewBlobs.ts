// Tải mỗi URL preview đúng một lần → blob URL cho mask-image; đếm tham chiếu, LRU cho blob không ai dùng.
export const MAX_BLOBS = 800;
/** Preview của font còn trên ổ cloud: tối đa bao nhiêu request cùng bay (mỗi cái có thể kéo cả file font về). */
export const MAX_CLOUD_FETCHES = 4;

export class PreviewHttpError extends Error {
  constructor(public status: number, public code: string) {
    super(`${status} ${code}`);
    this.name = 'PreviewHttpError';
  }
}

type Resp = { ok: boolean; status: number; blob(): Promise<Blob>; text(): Promise<string> };
export type BlobDeps = {
  fetch(url: string, init: { signal: AbortSignal }): Promise<Resp>;
  createObjectURL(b: Blob): string;
  revokeObjectURL(u: string): void;
};

type Entry = { refs: number; promise: Promise<string>; blobUrl?: string; ctrl: AbortController; lastRelease: number; cloud: boolean; started: boolean };
type Waiter = { go(): void; fail(e: unknown): void };

const defaultDeps = (): BlobDeps => ({
  fetch: (u, init) => globalThis.fetch(u, init),
  createObjectURL: b => URL.createObjectURL(b),
  revokeObjectURL: u => URL.revokeObjectURL(u),
});

export class PreviewBlobs {
  private entries = new Map<string, Entry>();
  private clock = 0;
  private fetches = 0;
  private revoked = 0;
  private deps: BlobDeps;
  private cloudActive = 0;
  private cloudQueue: Waiter[] = [];

  constructor(deps?: BlobDeps, private max = MAX_BLOBS) {
    this.deps = deps ?? defaultDeps();
  }

  /**
   * `cloud`: font của thẻ còn là placeholder trên ổ cloud — đi qua làn riêng (≤ MAX_CLOUD_FETCHES cùng lúc).
   * Thẻ cuộn đi khi còn xếp hàng thì bị huỷ hẳn (không fetch, không giữ chỗ); đã bắt đầu tải thì để tải nốt
   * (backend không huỷ được, và kết quả vào cache cho lần cuộn lại) thay vì bỏ lại request mồ côi.
   */
  acquire(url: string, opts: { cloud?: boolean } = {}): Promise<string> {
    const hit = this.entries.get(url);
    if (hit) { hit.refs++; return hit.promise; }
    const ctrl = new AbortController();
    const entry: Entry = { refs: 1, ctrl, lastRelease: 0, promise: Promise.resolve(''), cloud: !!opts.cloud, started: false };
    entry.promise = (async () => {
      if (entry.cloud) await this.takeCloudSlot(ctrl.signal);
      entry.started = true;
      this.fetches++;
      try {
        const res = await this.deps.fetch(url, { signal: ctrl.signal });
        if (!res.ok) {
          const code = await res.text().catch(() => '');
          if (this.entries.get(url) === entry) this.entries.delete(url); // lỗi không cache
          throw new PreviewHttpError(res.status, code);
        }
        const blob = await res.blob();
        if (ctrl.signal.aborted) throw new DOMException('aborted', 'AbortError');
        entry.blobUrl = this.deps.createObjectURL(blob);
        if (entry.refs === 0) { entry.lastRelease = ++this.clock; this.trim(); } // thẻ đã đi mất: giữ như blob rảnh
        return entry.blobUrl;
      } finally {
        if (entry.cloud) this.freeCloudSlot();
      }
    })();
    this.entries.set(url, entry);
    return entry.promise;
  }

  private takeCloudSlot(signal: AbortSignal): Promise<void> {
    if (this.cloudActive < MAX_CLOUD_FETCHES) { this.cloudActive++; return Promise.resolve(); }
    return new Promise<void>((resolve, reject) => {
      const w: Waiter = { go: () => { signal.removeEventListener('abort', onAbort); resolve(); }, fail: reject };
      const onAbort = () => {
        const i = this.cloudQueue.indexOf(w);
        if (i >= 0) this.cloudQueue.splice(i, 1);
        reject(new DOMException('aborted', 'AbortError'));
      };
      signal.addEventListener('abort', onAbort, { once: true });
      this.cloudQueue.push(w);
    });
  }

  private freeCloudSlot() {
    const next = this.cloudQueue.shift();
    if (next) next.go(); // chỗ trống chuyển thẳng cho người xếp hàng đầu (cloudActive giữ nguyên)
    else this.cloudActive = Math.max(0, this.cloudActive - 1);
  }

  release(url: string) {
    const e = this.entries.get(url);
    if (!e) return;
    e.refs = Math.max(0, e.refs - 1);
    if (e.refs > 0) return;
    if (!e.blobUrl) {
      if (e.cloud && e.started) return; // đang tải từ cloud: để tải nốt, xong sẽ thành blob rảnh (xem acquire)
      e.ctrl.abort(); // chưa xong và không ai cần → huỷ
      this.entries.delete(url);
      return;
    }
    e.lastRelease = ++this.clock;
    this.trim();
  }

  private trim() {
    const idle = [...this.entries.entries()].filter(([, e]) => e.refs === 0 && e.blobUrl);
    if (idle.length <= this.max) return;
    idle.sort((a, b) => a[1].lastRelease - b[1].lastRelease);
    for (const [url, e] of idle.slice(0, idle.length - this.max)) {
      this.deps.revokeObjectURL(e.blobUrl!);
      this.entries.delete(url);
      this.revoked++;
    }
  }

  stats() {
    return { entries: this.entries.size, fetches: this.fetches, revoked: this.revoked };
  }
}
