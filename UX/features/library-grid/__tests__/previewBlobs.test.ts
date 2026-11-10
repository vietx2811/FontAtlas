import { describe, expect, it, vi } from 'vitest';
import { PreviewBlobs, PreviewHttpError, type BlobDeps } from '../previewBlobs';

function deps(status = 200) {
  let n = 0; const revoked: string[] = []; const aborted: string[] = [];
  const d: BlobDeps = {
    fetch: vi.fn(async (url: string, init: { signal: AbortSignal }) => {
      init.signal.addEventListener('abort', () => aborted.push(url));
      return { ok: status === 200, status, blob: async () => new Blob(['x']), text: async () => 'stale_generation' };
    }),
    createObjectURL: () => `blob:${++n}`,
    revokeObjectURL: u => void revoked.push(u),
  };
  return { d, revoked, aborted };
}

describe('PreviewBlobs', () => {
  it('fetches each url once and shares the blob url', async () => {
    const { d } = deps(); const b = new PreviewBlobs(d);
    const [x, y] = await Promise.all([b.acquire('/a'), b.acquire('/a')]);
    expect(x).toBe(y);
    expect(d.fetch).toHaveBeenCalledTimes(1);
  });

  it('keeps referenced blobs and revokes only unreferenced beyond the cap', async () => {
    const { d, revoked } = deps(); const b = new PreviewBlobs(d, 2);
    const a = await b.acquire('/a');                               // giữ /a
    for (const u of ['/b', '/c', '/d']) { await b.acquire(u); b.release(u); }
    expect(revoked).not.toContain(a);
    expect(revoked).toHaveLength(1);                               // /b bị đẩy; /c /d còn trong 2 chỗ
    expect(b.stats().entries).toBe(3);
  });

  it('non-ok responses reject with status and code and are not cached', async () => {
    const { d } = deps(410); const b = new PreviewBlobs(d);
    await expect(b.acquire('/a')).rejects.toEqual(new PreviewHttpError(410, 'stale_generation'));
    await b.acquire('/a').catch(() => {});
    expect(d.fetch).toHaveBeenCalledTimes(2);
  });

  it('aborts the fetch when released before it finishes', async () => {
    const { d, aborted } = deps(); const b = new PreviewBlobs(d);
    const p = b.acquire('/a'); b.release('/a');
    await p.catch(() => {});
    expect(aborted).toEqual(['/a']);
  });
});

/** Fetch treo cho tới khi test cho chạy tiếp: đếm được bao nhiêu request đang bay cùng lúc. */
function gated() {
  const waiting: Array<{ url: string; open(status?: number): void; signal: AbortSignal }> = [];
  let n = 0; const revoked: string[] = [];
  const d: BlobDeps = {
    fetch: vi.fn((url: string, init: { signal: AbortSignal }) => new Promise(resolve => {
      waiting.push({ url, signal: init.signal, open: (status = 200) => resolve({ ok: status === 200, status, blob: async () => new Blob(['x']), text: async () => 'cloud_timeout' }) });
    })) as BlobDeps['fetch'],
    createObjectURL: () => `blob:${++n}`,
    revokeObjectURL: u => void revoked.push(u),
  };
  return { d, waiting, revoked };
}
const tick = async () => { for (let i = 0; i < 5; i++) await Promise.resolve(); };

describe('PreviewBlobs · cloud lane', () => {
  it('runs at most 4 cloud fetches at once; a finished one lets the next start', async () => {
    const { d, waiting } = gated(); const b = new PreviewBlobs(d);
    const ps = Array.from({ length: 10 }, (_, i) => b.acquire(`/c${i}`, { cloud: true }));
    await tick();
    expect(d.fetch).toHaveBeenCalledTimes(4);
    waiting[0].open(); await ps[0]; await tick();
    expect(d.fetch).toHaveBeenCalledTimes(5);
    expect(waiting[4].url).toBe('/c4');
  });

  it('local previews are not held back by a full cloud lane', async () => {
    const { d } = gated(); const b = new PreviewBlobs(d);
    for (let i = 0; i < 6; i++) void b.acquire(`/c${i}`, { cloud: true });
    for (let i = 0; i < 6; i++) void b.acquire(`/l${i}`);
    await tick();
    expect(d.fetch).toHaveBeenCalledTimes(4 + 6);
  });

  it('a card scrolled away while still queued is cancelled: never fetched, no slot used', async () => {
    const { d, waiting } = gated(); const b = new PreviewBlobs(d);
    const first = Array.from({ length: 4 }, (_, i) => b.acquire(`/c${i}`, { cloud: true }));
    const queued = b.acquire('/late', { cloud: true });
    const queued2 = b.acquire('/later', { cloud: true });
    await tick();
    b.release('/late');
    await expect(queued).rejects.toMatchObject({ name: 'AbortError' });
    waiting[0].open(); await first[0]; await tick();
    expect(waiting.map(w => w.url)).not.toContain('/late');
    expect(waiting.map(w => w.url)).toContain('/later');   // lượt trống chuyển cho thẻ còn đang hiện
    void queued2;
  });

  it('an in-flight cloud fetch is not aborted when the card leaves: it finishes and stays cached for later', async () => {
    const { d, waiting } = gated(); const b = new PreviewBlobs(d);
    const p = b.acquire('/c0', { cloud: true });
    await tick();
    b.release('/c0');
    expect(waiting[0].signal.aborted).toBe(false);
    waiting[0].open();
    await p;
    expect(b.stats().entries).toBe(1);
    const again = await b.acquire('/c0', { cloud: true });
    expect(again).toMatch(/^blob:/);
    expect(d.fetch).toHaveBeenCalledTimes(1);
  });

  it('a failed cloud fetch frees its slot and is reported with status and code', async () => {
    const { d, waiting } = gated(); const b = new PreviewBlobs(d);
    const ps = Array.from({ length: 5 }, (_, i) => b.acquire(`/c${i}`, { cloud: true }));
    ps.forEach(p => p.catch(() => {}));
    await tick();
    waiting[0].open(504);
    await expect(ps[0]).rejects.toEqual(new PreviewHttpError(504, 'cloud_timeout'));
    await tick();
    expect(d.fetch).toHaveBeenCalledTimes(5);
  });
});
