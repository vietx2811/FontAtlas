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
