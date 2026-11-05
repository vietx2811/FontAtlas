// Tải mỗi URL preview đúng một lần → blob URL cho mask-image; đếm tham chiếu, LRU cho blob không ai dùng.
export const MAX_BLOBS = 800;

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

type Entry = { refs: number; promise: Promise<string>; blobUrl?: string; ctrl: AbortController; lastRelease: number };

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

  constructor(deps?: BlobDeps, private max = MAX_BLOBS) {
    this.deps = deps ?? defaultDeps();
  }

  acquire(url: string): Promise<string> {
    const hit = this.entries.get(url);
    if (hit) { hit.refs++; return hit.promise; }
    const ctrl = new AbortController();
    const entry: Entry = { refs: 1, ctrl, lastRelease: 0, promise: Promise.resolve('') };
    this.fetches++;
    entry.promise = (async () => {
      const res = await this.deps.fetch(url, { signal: ctrl.signal });
      if (!res.ok) {
        const code = await res.text().catch(() => '');
        if (this.entries.get(url) === entry) this.entries.delete(url); // lỗi không cache
        throw new PreviewHttpError(res.status, code);
      }
      const blob = await res.blob();
      if (ctrl.signal.aborted) throw new DOMException('aborted', 'AbortError');
      entry.blobUrl = this.deps.createObjectURL(blob);
      return entry.blobUrl;
    })();
    this.entries.set(url, entry);
    return entry.promise;
  }

  release(url: string) {
    const e = this.entries.get(url);
    if (!e) return;
    e.refs = Math.max(0, e.refs - 1);
    if (e.refs > 0) return;
    if (!e.blobUrl) {
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
