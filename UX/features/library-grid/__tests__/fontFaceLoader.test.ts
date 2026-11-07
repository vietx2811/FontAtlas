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

  it('release evicts when over budget without waiting for the next load', async () => {
    const f = fake(); const l = new FontFaceLoader(f.deps, { BUDGET_BYTES: 1000 });
    const a = makeRow(1, { fileSize: 700 }), b = makeRow(2, { fileSize: 700 });
    l.acquire(a); l.acquire(b); await f.settle(); // cả hai đang dùng → 1400 > 1000
    expect(l.stats().readyBytes).toBe(1400);
    l.release(a);
    expect(l.stats()).toMatchObject({ readyBytes: 700, evictions: 1 });
  });

  it('dispose removes every face, cancels pending work and refuses new acquires', async () => {
    const f = fake(); const l = new FontFaceLoader(f.deps);
    const a = makeRow(1);
    l.acquire(a); await f.settle();
    const pending = l.acquire(makeRow(2)); // đang tải
    l.dispose();
    expect(f.added.size).toBe(0);
    expect(await pending).toBeNull();
    expect(await l.acquire(makeRow(3))).toBeNull();
    expect(f.sources).toHaveLength(2);
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

describe('FontFaceLoader: name lane (soft refs)', () => {
  const soft = { soft: true };

  it('names wait for the scroll to stop, even when slow enough for content to load; content is unaffected', () => {
    const f = fake({ velocity: LOADER.V_MAX - 1 }); const l = new FontFaceLoader(f.deps);
    l.onScroll();
    l.acquire(makeRow(1), soft);
    l.acquire(makeRow(2)); // nội dung: tải ngay vì chậm hơn V_MAX
    expect(f.sources).toHaveLength(1);
    f.setV(0); f.advance(LOADER.IDLE_MS);
    expect(f.sources).toHaveLength(2);
  });

  it('at most NAME_MAX_ACTIVE name loads at once, and they never take a hover/live/panel slot', () => {
    const f = fake(); const l = new FontFaceLoader(f.deps);
    for (let i = 0; i < 6; i++) l.acquire(makeRow(i), soft);
    expect(l.stats()).toMatchObject({ lowActive: LOADER.NAME_MAX_ACTIVE, active: 0 });
    expect(f.sources).toHaveLength(LOADER.NAME_MAX_ACTIVE);
    for (let i = 100; i < 100 + LOADER.MAX_ACTIVE; i++) l.acquire(makeRow(i)); // hover/live: đủ 6 slot dù làn tên đang bận
    expect(l.stats()).toMatchObject({ active: LOADER.MAX_ACTIVE, lowActive: LOADER.NAME_MAX_ACTIVE });
    expect(f.sources).toHaveLength(LOADER.NAME_MAX_ACTIVE + LOADER.MAX_ACTIVE);
  });

  it('a hard acquire of a face still queued for its name jumps to the high lane at once', () => {
    const f = fake(); const l = new FontFaceLoader(f.deps);
    const rows = [0, 1, 2, 3].map(i => makeRow(i));
    rows.forEach(r => l.acquire(r, soft));
    expect(f.sources).toHaveLength(2); // hai tên đang tải, hai tên chờ
    l.acquire(rows[3]);                // hover vào thẻ thứ 4
    expect(f.sources).toHaveLength(3);
    expect(l.stats()).toMatchObject({ active: 1, lowActive: 2 });
  });

  it('a face larger than NAME_MAX_BYTES is never loaded just for its name; content still loads it and then the name can use it', async () => {
    const f = fake(); const l = new FontFaceLoader(f.deps);
    const big = makeRow(1, { fileSize: LOADER.NAME_MAX_BYTES + 1 });
    expect(await l.acquire(big, soft)).toBeNull();
    expect(f.sources).toHaveLength(0);
    expect(l.stats().entries).toBe(0);
    const hard = l.acquire(big); await f.settle();
    expect(await hard).toBe(familyName(big));
    expect(await l.acquire(big, soft)).toBe(familyName(big)); // đã có sẵn → tên dùng luôn
  });

  it('over budget: faces held only by names are evicted (UI font fallback) before anything else; hard refs never are', async () => {
    const f = fake(); const l = new FontFaceLoader(f.deps, { BUDGET_BYTES: 250_000 });
    const content = makeRow(1, { fileSize: 200_000 }), name = makeRow(2, { fileSize: 200_000 });
    const a = l.acquire(content); await f.settle(); await a;
    const b = l.acquire(name, soft); await f.settle(); expect(await b).toBe(familyName(name));
    expect(l.stats()).toMatchObject({ evictions: 1, readyBytes: 200_000 });
    expect(f.added.size).toBe(1);
    expect(l.stats()).toMatchObject({ refs: 1, soft: 1 }); // ref của tên vẫn còn để release cân bằng
    l.release(name, soft);
    expect(l.stats()).toMatchObject({ refs: 1, soft: 0, entries: 1 });
    expect(f.added.size).toBe(1);
  });

  it('over budget with only hard refs: nothing in use is evicted', async () => {
    const f = fake(); const l = new FontFaceLoader(f.deps, { BUDGET_BYTES: 100 });
    l.acquire(makeRow(1, { fileSize: 200 })); l.acquire(makeRow(2, { fileSize: 200 })); await f.settle();
    expect(l.stats()).toMatchObject({ evictions: 0, readyBytes: 400 });
  });

  it('refs balance: every acquire released → no refs left, queued names are cancelled', async () => {
    const f = fake(); const l = new FontFaceLoader(f.deps);
    const rows = [0, 1, 2, 3, 4].map(i => makeRow(i));
    rows.forEach(r => l.acquire(r, soft)); l.acquire(rows[0]);
    expect(l.stats()).toMatchObject({ refs: 1, soft: 5 });
    rows.forEach(r => l.release(r, soft)); l.release(rows[0]);
    expect(l.stats()).toMatchObject({ refs: 0, soft: 0 });
    expect(l.stats().cancels).toBe(3); // 3 tên còn chờ trong làn thấp bị huỷ (2 cái đang tải thì để tải xong)
  });

  it('an evicted face that someone needs again is reloaded', async () => {
    const f = fake(); const l = new FontFaceLoader(f.deps, { BUDGET_BYTES: 250_000 });
    const content = makeRow(1, { fileSize: 200_000 }), name = makeRow(2, { fileSize: 200_000 });
    l.acquire(content); await f.settle();
    l.acquire(name, soft); await f.settle();
    expect(f.added.size).toBe(1);
    const again = l.acquire(name); await f.settle(); // hover vào thẻ → ref cứng
    expect(await again).toBe(familyName(name));
    expect(f.sources.filter(x => x.includes(name.hashHex))).toHaveLength(2);
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
