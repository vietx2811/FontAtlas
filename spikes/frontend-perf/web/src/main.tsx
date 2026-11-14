// SPIKE — code bỏ đi. 3 chế độ hiển thị lưới font + bench tự động.
// ?mode=a|b|c|csvg &n=2000 &auto=1 &tag=chromium
//  a    : DOM đầy đủ, mỗi thẻ 1 IntersectionObserver, readFile→Blob→FontFace, không dedupe/không gỡ (như UX/ hiện tại)
//  b    : react-window v2 List + FontFace cache (Map, 6 đồng thời, huỷ khi unmount, LRU 400 gỡ khỏi document.fonts)
//  c    : react-window v2 List + PNG alpha-mask dựng sẵn; FontFace thật chỉ khi hover/chọn
//  csvg : như c nhưng ảnh SVG path
import { memo, useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { createRoot } from 'react-dom/client';
import { List, type RowComponentProps } from 'react-window';

const P = new URLSearchParams(location.search);
const MODE = P.get('mode') ?? 'b';
const N = +(P.get('n') ?? 2000);
const AUTO = P.get('auto') === '1';
const COLS = 4, ROW_H = 120, PX = 48;
const TEXT0 = 'Hamburgefonts Ăn phở', TEXT1 = 'Đổi chữ mẫu 2026';

type Face = { id: number; family: string; style: string };
let pool: Face[] = [];
const face = (i: number) => pool[i % pool.length];

// ---- text store (useSyncExternalStore) ----
let text = TEXT0, textVer = 0;
const subs = new Set<() => void>();
const setText = (t: string) => { text = t; textVer++; subs.forEach(f => f()); };
const useText = () => useSyncExternalStore(cb => (subs.add(cb), () => subs.delete(cb)), () => text);

// ---- readiness registry: readyVer[i] = textVer khi thẻ i đã hiển thị đúng font + đúng chữ ----
let readyVer = new Int32Array(0);
const stats = { fontLoads: 0, fontBytes: 0, evictions: 0, cancels: 0, errors: 0 };

// ---- (a) loader như useFontLoader hiện tại ----
async function loadNaive(i: number): Promise<string> {
  const buf = await (await fetch(`/font/${face(i).id}`)).arrayBuffer();
  stats.fontLoads++; stats.fontBytes += buf.byteLength;
  const url = URL.createObjectURL(new Blob([buf])); // không revoke (giống code hiện tại)
  const ff = new FontFace(`fa-${i}`, `url("${url}")`);
  try { await ff.load(); } catch { stats.errors++; return 'serif'; } // font bị engine từ chối → fallback
  document.fonts.add(ff);
  return `fa-${i}`;
}

// ---- (b) cache: dedupe + 6 đồng thời + huỷ + LRU ----
type Entry = { state: 'waiting' | 'queued' | 'loading' | 'ready' | 'error'; refs: number; used: number; bytes: number; face?: FontFace; p: Promise<string>; resolve: (s: string) => void };
const cache = new Map<number, Entry>();
const queue: number[] = [];
let active = 0;
const MAX_ACTIVE = 6, LRU_CAP = 400;
// b2: ngân sách theo byte thay vì số lượng + chỉ bắt đầu tải khi ngừng cuộn IDLE ms (bỏ qua thẻ lướt qua)
const B2 = MODE === 'b2' || MODE.startsWith('c'), BYTE_BUDGET = 128 << 20, IDLE = 120;
// tải khi vận tốc cuộn < V_MAX px/s, hoặc khi đã ngừng cuộn IDLE ms
const V_MAX = 1500;
let readyBytes = 0, lastScroll = -1e9, lastTop = 0, velocity = 0, idleTimer = 0;
const canLoad = () => performance.now() - lastScroll > IDLE || velocity < V_MAX;
function flushWaiting() { for (const [i, e] of cache) if (e.state === 'waiting' && e.refs > 0) { e.state = 'queued'; queue.push(i); } pump(); }
addEventListener('scroll', ev => {
  const now = performance.now(), top = (ev.target as HTMLElement).scrollTop ?? 0;
  velocity = 0.7 * velocity + 0.3 * Math.abs(top - lastTop) / Math.max(1, now - lastScroll) * 1000;
  lastTop = top; lastScroll = now;
  clearTimeout(idleTimer); idleTimer = setTimeout(flushWaiting, IDLE);
  if (velocity < V_MAX) flushWaiting();
}, true);
function pump() {
  while (active < MAX_ACTIVE && queue.length) {
    const i = queue.shift()!; const e = cache.get(i)!;
    e.state = 'loading'; active++;
    fetch(`/font/${face(i).id}`).then(r => r.arrayBuffer()).then(async buf => {
      stats.fontLoads++; stats.fontBytes += buf.byteLength;
      const ff = new FontFace(`fb-${i}`, buf);
      await ff.load(); document.fonts.add(ff);
      e.face = ff; e.bytes = buf.byteLength; readyBytes += e.bytes; e.state = 'ready'; e.resolve(`fb-${i}`); evict();
    }).catch(() => { e.state = 'error'; stats.errors++; e.resolve('serif'); }).finally(() => { active--; pump(); });
  }
}
function evict() {
  if (B2) {
    const victims = [...cache.entries()].filter(([, e]) => e.state === 'ready' && e.refs === 0).sort((x, y) => x[1].used - y[1].used);
    while (readyBytes > BYTE_BUDGET && victims.length) { const [i, e] = victims.shift()!; document.fonts.delete(e.face!); readyBytes -= e.bytes; cache.delete(i); stats.evictions++; }
    return;
  }
  const ready = [...cache.entries()].filter(([, e]) => e.state === 'ready');
  if (ready.length <= LRU_CAP) return;
  ready.filter(([, e]) => e.refs === 0).sort((x, y) => x[1].used - y[1].used)
    .slice(0, ready.length - LRU_CAP).forEach(([i, e]) => { document.fonts.delete(e.face!); cache.delete(i); stats.evictions++; });
}
function acquire(i: number): Promise<string> {
  let e = cache.get(i);
  if (!e) {
    let resolve!: (s: string) => void;
    const p = new Promise<string>(r => (resolve = r));
    e = { state: B2 ? 'waiting' : 'queued', refs: 0, used: 0, bytes: 0, p, resolve };
    cache.set(i, e);
    if (B2 && !canLoad()) { /* chờ flushWaiting */ }
    else { e.state = 'queued'; queue.push(i); pump(); }
  }
  e.refs++; e.used = performance.now();
  return e.p;
}
function release(i: number) {
  const e = cache.get(i); if (!e) return;
  e.refs--;
  if (e.refs <= 0 && e.state === 'waiting') { cache.delete(i); stats.cancels++; return; }
  if (e.refs <= 0 && e.state === 'queued') { queue.splice(queue.indexOf(i), 1); cache.delete(i); stats.cancels++; }
}

// ---- cards ----
const cardStyle: React.CSSProperties = { flex: 1, minWidth: 0, margin: 4, padding: '8px 10px', background: '#fff', border: '1px solid #e7e5e4', borderRadius: 8, overflow: 'hidden', boxSizing: 'border-box' };
const Name = ({ i }: { i: number }) => <div style={{ fontSize: 11, color: '#78716c', whiteSpace: 'nowrap' }}>{i} · {face(i).family} {face(i).style}</div>;
const previewStyle = (fam: string | null): React.CSSProperties => ({ fontFamily: fam ?? 'sans-serif', fontSize: PX / 2 * 1.5, lineHeight: '64px', whiteSpace: 'nowrap', color: fam ? '#1c1917' : '#d6d3d1' });

function CardA({ i }: { i: number }) {
  const ref = useRef<HTMLDivElement>(null);
  const [fam, setFam] = useState<string | null>(null);
  const t = useText();
  useEffect(() => { // 1 observer / thẻ như FontCard.tsx
    const io = new IntersectionObserver(([en]) => { if (en.isIntersecting) { io.disconnect(); loadNaive(i).then(setFam); } }, { rootMargin: '200px' });
    io.observe(ref.current!); return () => io.disconnect();
  }, [i]);
  useEffect(() => { if (fam) readyVer[i] = textVer; }, [fam, t, i]);
  return <div ref={ref} style={cardStyle}><Name i={i} /><div style={previewStyle(fam)}>{t}</div></div>;
}

const CardB = memo(function CardB({ i }: { i: number }) {
  const [fam, setFam] = useState<string | null>(cache.get(i)?.state === 'ready' ? `fb-${i}` : null);
  const t = useText();
  useEffect(() => { let alive = true; acquire(i).then(f => alive && setFam(f)); return () => { alive = false; release(i); }; }, [i]);
  useEffect(() => { if (fam) readyVer[i] = textVer; }, [fam, t, i]);
  return <div style={cardStyle}><Name i={i} /><div style={previewStyle(fam)}>{t}</div></div>;
});

const CardC = memo(function CardC({ i, fmt }: { i: number; fmt: 'png' | 'svg' }) {
  const t = useText();
  const [hoverFam, setHoverFam] = useState<string | null>(null);
  const id = face(i).id;
  const src = t === TEXT0 ? `/${fmt}/${id}.${fmt}?c=${i}` : `/render/${id}.png?c=${i}&px=${PX}&text=${encodeURIComponent(t)}`;
  const v = textVer;
  return (
    <div style={cardStyle} onMouseEnter={() => acquire(i).then(setHoverFam)} onMouseLeave={() => { release(i); setHoverFam(null); }}>
      <Name i={i} />
      {hoverFam ? <div style={previewStyle(hoverFam)}>{t}</div>
        : <img key={src} src={src} decoding="async" width={320} height={56} style={{ display: 'block', height: 56, width: 320 }} alt=""
               onLoad={() => { readyVer[i] = v; }} onError={() => { readyVer[i] = v; stats.errors++; }} />}
    </div>
  );
});

function Card({ i }: { i: number }) {
  if (MODE === 'a') return <CardA i={i} />;
  if (MODE === 'b' || MODE === 'b2') return <CardB i={i} />;
  return <CardC i={i} fmt={MODE === 'csvg' ? 'svg' : 'png'} />;
}
const RowCells = ({ row }: { row: number }) => <>{Array.from({ length: COLS }, (_, c) => row * COLS + c).filter(i => i < N).map(i => <Card key={i} i={i} />)}</>;

function Row({ index, style }: RowComponentProps) {
  return <div style={{ ...style, display: 'flex' }}><RowCells row={index} /></div>;
}

function App() {
  const scroller = useRef<HTMLDivElement>(null);
  const rows = Math.ceil(N / COLS);
  useEffect(() => {
    const el = (MODE === 'a' ? scroller.current! : scroller.current!.firstElementChild) as HTMLDivElement;
    (window as any).__scroller = el;
    if (AUTO) bench(el);
  }, []);
  if (MODE === 'a')
    return <div ref={scroller} style={{ height: '100vh', overflow: 'auto' }}>
      {Array.from({ length: rows }, (_, r) => <div key={r} style={{ height: ROW_H, display: 'flex' }}><RowCells row={r} /></div>)}
    </div>;
  return <div ref={scroller}><List style={{ height: '100vh' }} rowComponent={Row} rowCount={rows} rowHeight={ROW_H} rowProps={{}} overscanCount={2} /></div>;
}

// ---- bench ----
const raf = () => new Promise<number>(r => requestAnimationFrame(r));
const sleep = (ms: number) => new Promise(r => setTimeout(r, ms));
function visibleIdx(el: HTMLElement) {
  const r0 = Math.floor(el.scrollTop / ROW_H), r1 = Math.min(Math.ceil((el.scrollTop + el.clientHeight) / ROW_H), Math.ceil(N / COLS));
  const out: number[] = [];
  for (let r = r0; r < r1; r++) for (let c = 0; c < COLS; c++) if (r * COLS + c < N) out.push(r * COLS + c);
  return out;
}
async function waitReady(el: HTMLElement, timeout = 60000) {
  const t0 = performance.now();
  while (performance.now() - t0 < timeout) {
    if (visibleIdx(el).every(i => readyVer[i] >= textVer)) { await raf(); await raf(); return performance.now(); }
    await raf();
  }
  return NaN;
}
const hud = (o: unknown) => { document.getElementById('hud')!.textContent = JSON.stringify(o, null, 1); };
const mem = () => ({ heap_mb: (performance as any).memory ? +((performance as any).memory.usedJSHeapSize / 1048576).toFixed(1) : null, fontfaces: document.fonts.size, dom: document.getElementsByTagName('*').length });

async function bench(el: HTMLElement) {
  const tMount = performance.now();
  const tFirst = await waitReady(el);
  const afterFirst = { ...mem(), loads: stats.fontLoads };
  await sleep(500);
  // cuộn nhanh 4000 px/s trong 6 s, đo khoảng cách frame + tỉ lệ thẻ trống trong viewport
  const SPEED = 4000, DUR = 6000;
  const frames: number[] = []; let blank = 0, seen = 0;
  let last = await raf(); const start = last;
  while (last - start < DUR) {
    const now = await raf(); frames.push(now - last); last = now;
    el.scrollTop = Math.min(el.scrollTop + SPEED * frames[frames.length - 1] / 1000, el.scrollHeight - el.clientHeight);
    for (const i of visibleIdx(el)) { seen++; if (readyVer[i] < textVer) blank++; }
  }
  const sorted = [...frames].sort((a, b) => a - b);
  // pha 2: duyệt chậm 600 px/s trong 6 s (người dùng đọc lướt)
  const loadsBeforeSlow = stats.fontLoads;
  const sframes: number[] = []; let sblank = 0, sseen = 0;
  last = await raf(); const sstart = last;
  while (last - sstart < DUR) {
    const now = await raf(); sframes.push(now - last); last = now;
    el.scrollTop = Math.min(el.scrollTop + 600 * sframes[sframes.length - 1] / 1000, el.scrollHeight - el.clientHeight);
    for (const i of visibleIdx(el)) { sseen++; if (readyVer[i] < textVer) sblank++; }
  }
  const settle0 = performance.now(); const tSettle = await waitReady(el); // thời gian lấp đầy sau khi dừng cuộn
  await sleep(1500);
  const afterScroll = { ...mem(), loads: stats.fontLoads, font_mb: +(stats.fontBytes / 1048576).toFixed(1), evictions: stats.evictions, cancels: stats.cancels, errors: stats.errors };
  // đổi chữ mẫu
  const t0 = performance.now(); setText(TEXT1); const tText = await waitReady(el);
  // hover → FontFace thật cho 1 thẻ đang thấy (chế độ c), đo riêng
  let hover_ms = null;
  if (MODE.startsWith('c')) { const i = visibleIdx(el)[5]; const h0 = performance.now(); await acquire(i); hover_ms = +(performance.now() - h0).toFixed(1); release(i); }
  const res = {
    tag: P.get('tag') ?? (navigator.userAgent.includes('Chrome') ? 'chromium' : 'webkit'), mode: MODE, n: N, pool: pool.length,
    mount_ms: +tMount.toFixed(0), first_ms: +tFirst.toFixed(0), after_first: afterFirst,
    fps: +(frames.length / (DUR / 1000)).toFixed(1), frame_p50: +sorted[Math.floor(sorted.length * .5)].toFixed(1), frame_p95: +sorted[Math.floor(sorted.length * .95)].toFixed(1),
    frame_max: +sorted[sorted.length - 1].toFixed(0), jank_pct: +(100 * frames.filter(f => f > 33.4).length / frames.length).toFixed(1),
    blank_pct: +(100 * blank / seen).toFixed(1),
    slow_fps: +(sframes.length / (DUR / 1000)).toFixed(1), slow_frame_p95: +[...sframes].sort((a, b) => a - b)[Math.floor(sframes.length * .95)].toFixed(1),
    slow_blank_pct: +(100 * sblank / sseen).toFixed(1), slow_loads: stats.fontLoads - loadsBeforeSlow, settle_ms: +(tSettle - settle0).toFixed(0),
    after_scroll: afterScroll, text_change_ms: +(tText - t0).toFixed(0), hover_ms, ua: navigator.userAgent,
  };
  hud(res);
  (window as any).__result = res;
  await fetch('/api/result', { method: 'POST', body: JSON.stringify(res) });
}

fetch('/api/manifest').then(r => r.json()).then((m: Face[]) => {
  pool = m; readyVer = new Int32Array(N).fill(-1);
  createRoot(document.getElementById('root')!).render(<App />);
});
(window as any).__dbg = { cache, stats, queue, get active() { return active; }, get lastScroll() { return lastScroll; } };
