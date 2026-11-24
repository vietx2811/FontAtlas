// Tự kiểm «cloud tải xong» (07): đo trong cửa sổ app DEBUG. Móc fetch (lệnh IPC của app đi qua fetch ipc://), kênh scan riêng, bộ thăm dò DOM 50 ms.
const T = window.__TAURI_INTERNALS__; const raw = T.invoke.bind(T); const T0 = window.__Q8_T0;
const now = () => Date.now(); const sleep = ms => new Promise(r => setTimeout(r, ms));
const wait = async (fn, ms = 20000) => { const t = now(); while (now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) {} await sleep(150); } return null; };
let nlog = 0; const SEQ = (+sessionStorage.getItem('q8seq') || 0);
const log = async (step, ok, detail) => { nlog++; try { await raw('set_setting', { key: 'ui.qa.' + String(nlog).padStart(3, '0'), value: JSON.stringify({ step, ok, detail, t: Math.round((now() - T0) / 100) / 10 }) }); } catch (e) {} };
const logBig = async (step, arr) => { for (let i = 0; i < arr.length; i += 25) await log(step + '#' + (i / 25), true, arr.slice(i, i + 25)); };
const ch = cb => '__CHANNEL__:' + T.transformCallback(x => { try { cb(x && x.message !== undefined ? x.message : x); } catch (e) {} });
const E = e => { try { return (e && e.message ? e.message + ' | ' : '') + ((e && e.stack) ? e.stack : JSON.stringify(e)); } catch (x) { return String(e); } };
// --- IPC theo thời gian
const ipc = []; const of_ = window.fetch;
window.fetch = function (u, ...r) { try { const m = String(u && u.url || u).match(/^(?:https?:\/\/ipc\.localhost|ipc:\/\/localhost)\/([^?]+)/); if (m) { const ent = [now(), decodeURIComponent(m[1]), null]; ipc.push(ent); const p = of_.apply(this, [u, ...r]); p.then(() => { ent[2] = now() - ent[0]; }, () => { ent[2] = -1; }); return p; } } catch (e) {} return of_.apply(this, [u, ...r]); };
const REL = ['query_faces', 'list_sources', 'list_collections', 'list_tags', 'duplicate_summary', 'list_cases', 'name_clashes', 'scan_status'];
const ipcCount = (from, to = Infinity) => { const o = {}; ipc.filter(x => x[0] >= from && x[0] < to && REL.includes(x[1])).forEach(x => o[x[1]] = (o[x[1]] || 0) + 1); return o; };
// --- sự kiện scan (kênh của riêng mình, app vẫn có kênh của nó)
const scanEv = [];
const scanSub = async () => { // KHÔNG gọi scan_subscribe (kênh 1 người nghe: cướp sự kiện của giao diện beta.5). Suy lượt quét từ TaskHub (list_tasks mỗi 60 ms)
  const seen = new Map();
  setInterval(async () => { try { const l = await raw('list_tasks', {}); const cur = new Set(); (l.tasks || []).forEach(t => { if (t.kind !== 'scan') return; cur.add(t.id); if (!seen.has(t.id)) { seen.set(t.id, now()); scanEv.push([now(), 'queued', null, t.trigger]); } }); for (const id of [...seen.keys()]) if (!cur.has(id)) { seen.delete(id); scanEv.push([now(), 'finished', null, null]); } } catch (e) {} }, 60);
  return true; };
const evCount = (from, to = Infinity) => { const o = {}; scanEv.filter(x => x[0] >= from && x[0] < to).forEach(x => o[x[1]] = (o[x[1]] || 0) + 1); return o; };
// --- DOM
const side = () => document.querySelector('[data-testid=sidebar]');
const names = () => document.querySelectorAll('[data-testid="face-name"]').length;
const searchVal = () => document.querySelector('input[aria-label="Search fonts"]')?.value ?? null;
const sideInfo = () => { const s = side(); const tx = s ? s.innerText : ''; const m = tx.match(/All\n(\d+)/); const d = tx.match(/(?:Duplicates?|Duplicate fonts)\n(\d+)/i); return { has: !!s, all: m ? +m[1] : null, dups: d ? +d[1] : null, srcs: (tx.match(/Sources\n([\s\S]*?)\nTags/) || [, ''])[1].split('\n').filter(x => x && !/^\d+$/.test(x)), scanning: !!document.querySelector('.fa-scan'), txt: tx }; };
const blank = []; const tl = []; let lastKey = ''; let scroller = null, remounts = 0, scrollerId = 0;
const poll = setInterval(() => {
  const sc = document.querySelector('[data-testid="library-grid-scroller"]');
  if (sc !== scroller) { if (sc && scroller) remounts++; scroller = sc; }
  const si = sideInfo(); const n = names(); const sv = searchVal();
  const key = [n > 0 ? 'cards' : 'ZERO', !!sc, sv === '' ? 'q-empty' : 'q:' + sv, si.all, si.dups, si.srcs.length, si.scanning].join('|');
  if (n === 0 && sv === '' && (si.all || 0) > 0 && blank.length < 3) { const m = document.querySelector('main, .fa-shell-main, [data-testid=window-shell]'); blank.push({ t: now() - T0, scroller: !!sc, scrollerKids: sc ? sc.children.length : null, scrollerH: sc ? sc.scrollHeight : null, scrollTop: sc ? sc.scrollTop : null, viewCount: document.querySelector('[data-testid=view-count]')?.innerText, nonSidebarText: (m ? m.innerText : document.body.innerText).replace(si.txt, '').slice(0, 300) }); }
  if (key !== lastKey) { lastKey = key; tl.push([now(), n, !!sc, sv, si.all, si.dups, si.srcs.join(','), si.scanning]); }
}, 50);
// Khoảnh khắc lưới rỗng khi ô tìm rỗng và thư viện có font (All > 0): [bắt đầu, dài ms]
const emptyEpisodes = (from, to = Infinity) => { const out = []; let st = null, lastAll = 0;
  for (const [t, n, sc, sv, all] of tl) { if (t < from) { lastAll = all ?? lastAll; if (n === 0 && sv === '' && (all ?? lastAll) > 0) st = from; else st = null; continue; } if (t > to) break; if (all != null) lastAll = all;
    const bad = n === 0 && sv === '' && lastAll > 0; if (bad && st == null) st = t; if (!bad && st != null) { out.push([st, t - st]); st = null; } }
  if (st != null) out.push([st, Math.min(now(), to) - st]); return out; };
const sideBlank = (from, to = Infinity) => tl.filter(x => x[0] >= from && x[0] < to && x[6] === '').length;
const skipOnboarding = async () => { const btn = re => [...document.querySelectorAll('button')].find(b => re.test(b.textContent.trim()));
  for (let i = 0; i < 40; i++) { await sleep(500); if (!document.querySelector('.fa-onb')) { if (document.querySelector('.fa-splash,[data-testid=splash]')) continue; break; }
    ['Đã hiểu','Không dùng','Bỏ qua','Để sau','Tiếp tục','Bắt đầu','Vào thư viện','Got it','Skip','Later','Continue','Start'].map(t => btn(new RegExp('^' + t))).find(Boolean)?.click(); } };
const go = async name => { await log('q8-wait-' + name, true, { epochMs: now() }); };
const scanIdle = async (ms = 180000) => wait(async () => { const st = await raw('scan_status', {}); return !st.running && !st.scanning; }, ms);

const lat = (from, to = Infinity) => { const o = {}; ipc.filter(x => x[0] >= from && x[0] < to && REL.includes(x[1]) && x[2] != null).forEach(x => (o[x[1]] = o[x[1]] || []).push(x[2]));
  const r = {}; for (const k in o) { const a = o[k].sort((p, q) => p - q); r[k] = { n: a.length, p50: a[Math.floor(a.length / 2)], p95: a[Math.min(a.length - 1, Math.ceil(a.length * 0.95) - 1)], max: a[a.length - 1] }; } return r; };
