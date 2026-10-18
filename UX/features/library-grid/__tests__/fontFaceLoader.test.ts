import { describe, expect, it } from 'vitest';
import { makeRow } from '../api';
import { FontFaceLoader, LOADER, ScrollVelocity, familyName, type LoaderDeps } from '../fontFaceLoader';

/** Phụ thuộc giả: đồng hồ tay, load() giữ lại được, đếm add/delete và nguồn đã dùng. */
function fake(o: { velocity?: number; fail?: boolean } = {}) {
  let t = 0; let v = o.velocity ?? 0;
  const timers: Array<{ at: number; fn: () => void }> = [];
  const pending: Array<() => void> = [];
  const added = new Set<unknown>(); const sources: string[] = [];
  const deps: LoaderDeps = {
    createFace: (family, source) => { sources.push(source); return { family, load: () => new Promise((res, rej) => pending.push(() => (o.fail ? rej(new Error('x')) : res(null)))) }; },
    fonts: { add: f => void added.add(f), delete: f => added.delete(f) },
    fontUrl: r => (r.faceIndex > 0 ? null : `fontatlas-font://localhost/${r.hashHex}_0.font`),
    now: () => t, velocity: () => v,
    setTimer: (fn, ms) => { const h = { at: t + ms, fn }; timers.push(h); return h; },
    clearTimer: h => { const i = timers.indexOf(h as never); if (i >= 0) timers.splice(i, 1); },
  };
  const settle = async () => { while (pending.length) pending.shift()!(); for (let i = 0; i < 10; i++) await Promise.resolve(); };
  const advance = (ms: number) => { t += ms; for (const h of [...timers]) if (h.at <= t) { timers.splice(timers.indexOf(h), 1); h.fn(); } };
  return { deps, settle, advance, added, sources, setV: (x: number) => (v = x) };
}

describe('FontFaceLoader', () => {
  it('dedupes, loads by url (no ArrayBuffer) and returns the family name', async () => {
    const f = fake(); const l = new FontFaceLoader(f.deps);
    const r = makeRow(1);
    const a = l.acquire(r); const b = l.acquire(r);
    await f.settle();
    expect(await a).toBe(familyName(r));
    expect(await b).toBe(familyName(r));
    expect(f.sources).toEqual([`url("fontatlas-font://localhost/${r.hashHex}_0.font")`]);
  });

  it('refuses_face_index_above_zero', async () => {
    const f = fake(); const l = new FontFaceLoader(f.deps);
    expect(await l.acquire(makeRow(1, { faceIndex: 1 }))).toBeNull();
    expect(f.sources).toHaveLength(0);
  });

  it('caps concurrency at MAX_ACTIVE', () => {
    const f = fake(); const l = new FontFaceLoader(f.deps);
    for (let i = 0; i < 10; i++) l.acquire(makeRow(i));
    expect(l.stats().active).toBe(LOADER.MAX_ACTIVE);
    expect(f.sources).toHaveLength(LOADER.MAX_ACTIVE);
  });

  it('fast_scroll_defers_until_idle and cancels rows released meanwhile', () => {
    const f = fake({ velocity: 4000 }); const l = new FontFaceLoader(f.deps);
    const a = makeRow(1), b = makeRow(2);
    l.acquire(a); l.acquire(b); l.onScroll();
    expect(f.sources).toHaveLength(0);
    l.release(b);
    f.setV(0); f.advance(LOADER.IDLE_MS);
    expect(f.sources).toHaveLength(1);
    expect(l.stats().cancels).toBe(1);
  });

  it('slow scroll below V_MAX loads immediately', () => {
    const f = fake({ velocity: LOADER.V_MAX - 1 }); const l = new FontFaceLoader(f.deps);
    l.onScroll(); l.acquire(makeRow(1));
    expect(f.sources).toHaveLength(1);
  });

  it('evicts unreferenced faces by fileSize, oldest first, never in-use ones', async () => {
    const f = fake(); const l = new FontFaceLoader(f.deps, { BUDGET_BYTES: 1000 });
    const rs = [0, 1, 2, 3].map(i => makeRow(i, { fileSize: 400 }));
    l.acquire(rs[0]); await f.settle(); l.release(rs[0]); f.advance(1);
    l.acquire(rs[1]); await f.settle(); l.release(rs[1]); f.advance(1);
    l.acquire(rs[2]); await f.settle();
    l.acquire(rs[3]); await f.settle();
    expect(l.stats()).toMatchObject({ evictions: 2, readyBytes: 800 });
    expect(f.added.size).toBe(2);
  });

  it('load error resolves null and is not retried', async () => {
    const f = fake({ fail: true }); const l = new FontFaceLoader(f.deps);
    const r = makeRow(1);
    const p = l.acquire(r); await f.settle();
    expect(await p).toBeNull();
    l.release(r);
    expect(await l.acquire(r)).toBeNull();
    expect(f.sources).toHaveLength(1);
  });
});

describe('ScrollVelocity', () => {
  it('measures px/s and decays to 0 after idle', () => {
    const v = new ScrollVelocity();
    v.sample(0, 0); v.sample(400, 100); v.sample(800, 200);
    expect(v.value(200)).toBeGreaterThan(LOADER.V_MAX);
    expect(v.value(200 + LOADER.IDLE_MS + 1)).toBe(0);
  });
});
