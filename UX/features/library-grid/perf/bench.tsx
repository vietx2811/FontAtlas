// Harness S2: lưới thật (LibraryGrid) + mock API; kịch bản đo giống spike 05.
import { useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { createMockLibraryApi, makeRow } from '../api';
import { LibraryGrid, type PreviewMode } from '../LibraryGrid';

const P = new URLSearchParams(location.search);
const N = +(P.get('n') ?? 5000);
const MODE = (P.get('mode') ?? 'mask') as PreviewMode;
const AUTO = P.get('auto') === '1';
const TEXT1 = 'Đổi chữ mẫu 2026';

const raf = () => new Promise<number>(r => requestAnimationFrame(r));
const sleep = (ms: number) => new Promise(r => setTimeout(r, ms));
const visibleCells = (el: HTMLElement) => {
  const box = el.getBoundingClientRect();
  return [...el.querySelectorAll<HTMLElement>('[role=gridcell]')].filter(c => {
    const r = c.getBoundingClientRect(); return r.bottom > box.top && r.top < box.bottom;
  });
};
const allReady = (el: HTMLElement) => { const v = visibleCells(el); return v.length > 0 && v.every(c => c.dataset.state === 'ready'); };
async function waitReady(el: HTMLElement, timeout = 60000) {
  const t0 = performance.now();
  while (performance.now() - t0 < timeout) { if (allReady(el)) { await raf(); await raf(); return performance.now(); } await raf(); }
  return NaN;
}
async function scrollPhase(el: HTMLElement, speed: number, dur: number) {
  const frames: number[] = []; let blank = 0, seen = 0;
  let last = await raf(); const start = last;
  while (last - start < dur) {
    const now = await raf(); frames.push(now - last); last = now;
    el.scrollTop = Math.min(el.scrollTop + speed * frames[frames.length - 1] / 1000, el.scrollHeight - el.clientHeight);
    for (const c of visibleCells(el)) { seen++; if (c.dataset.state !== 'ready') blank++; }
  }
  const s = [...frames].sort((a, b) => a - b);
  return { fps: frames.length / (dur / 1000), p95: s[Math.floor(s.length * 0.95)], blank_pct: 100 * blank / Math.max(1, seen) };
}

async function bench(setText: (t: string) => void) {
  let el: HTMLElement | null = null;
  while (!(el = document.querySelector<HTMLElement>('[data-testid=library-grid-scroller]'))) await raf();
  const first_ms = await waitReady(el);
  await sleep(500);
  const fast = await scrollPhase(el, 4000, 6000);
  const slow = await scrollPhase(el, 600, 6000);
  await waitReady(el);
  await sleep(1500);
  const t0 = performance.now();
  setText(TEXT1);
  await raf(); await raf(); // để ô chuyển sang 'loading' trước khi chờ
  const text_change_ms = (await waitReady(el)) - t0;
  const r = {
    mode: MODE, n: N, first_ms: Math.round(first_ms),
    fps: +fast.fps.toFixed(1), frame_p95: +fast.p95.toFixed(1), blank_pct: +fast.blank_pct.toFixed(1),
    slow_fps: +slow.fps.toFixed(1), slow_blank_pct: +slow.blank_pct.toFixed(1),
    text_change_ms: Math.round(text_change_ms), fontfaces: document.fonts.size,
    dom: document.getElementsByTagName('*').length, ua: navigator.userAgent,
  };
  (window as any).__result = r;
  await fetch('/api/result', { method: 'POST', body: JSON.stringify(r) });
}

function App({ rows }: { rows: ReturnType<typeof makeRow>[] }) {
  const [api] = useState(() => createMockLibraryApi(rows, { toUrl: name => `/preview/${name}` }));
  const [text, setText] = useState('Hamburgefonts Ăn phở');
  useEffect(() => { if (AUTO) bench(setText); }, []);
  return <LibraryGrid api={api} query={{}} width={1280} height={800} previewMode={MODE} sampleText={text} />;
}

fetch('/api/manifest').then(r => r.json()).then((m: Array<{ family: string }>) => {
  const rows = Array.from({ length: N }, (_, i) => makeRow(i, { family: m[i % m.length].family || `Face ${i}` }));
  createRoot(document.getElementById('root')!).render(<App rows={rows} />);
});
