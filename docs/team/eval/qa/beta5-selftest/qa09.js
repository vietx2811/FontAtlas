// Tự kiểm beta.5 (07 QA/DevOps): khay tác vụ trên bản DEBUG, dữ liệu tạm trên VXData, chỉ font fixture. __Q__ do run09.sh thay.
// Kết quả ghi vào bảng settings (ui.qa.NNN); điểm dừng cho shell: bước "q9-wait-<tên>".
(async () => {
  const T = window.__TAURI_INTERNALS__;
  const inv = (c, a) => T.invoke(c, a);
  const T0 = Date.now(); let n = 0;
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const wait = async (fn, ms = 20000, step = 100) => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) {} await sleep(step); } return null; };
  const log = async (step, ok, detail) => { n++; try { await inv('set_setting', { key: 'ui.qa.' + String(n).padStart(3, '0'), value: JSON.stringify({ step, ok, detail, t: Math.round((Date.now() - T0) / 1000) }) }); } catch (e) {} };
  const ch = cb => '__CHANNEL__:' + T.transformCallback(x => { try { cb(x && x.message !== undefined ? x.message : x); } catch (e) {} });
  const quiet = () => '__CHANNEL__:' + T.transformCallback(() => {});
  const keyOf = r => ({ hashHex: r.hashHex ?? r.hash_hex, faceIndex: r.faceIndex ?? r.face_index ?? 0 });
  const mkq = (text, sourceIds, extra = {}) => ({ text, vi: null, license: [], variable: null, italic: null, sourceIds, favoritesOnly: false, collectionId: null, tagIds: [], includeMissing: false, sort: 'family', descending: false, offset: 0, limit: 2000, axisFilter: [], ...extra });
  const E = e => { try { return (e && e.message ? e.message + ' | ' : '') + ((e && e.stack) ? String(e.stack).slice(0, 300) : JSON.stringify(e)); } catch (x) { return String(e); } };
  const q = s => document.querySelector(s);
  const qa = s => [...document.querySelectorAll(s)];
  const snap = async name => { try { document.getAnimations().forEach(x => { try { x.finish(); } catch (e) {} }); } catch (e) {} await sleep(120); try { await inv('plugin:event|emit', { event: 'qa-snap', payload: name }); } catch (e) {} await sleep(600); };
  const go = async (name, settle = 0) => { await log('q9-wait-' + name, true, null); if (settle) await sleep(settle); };
  const srcByDir = async d => (await inv('list_sources', {})).find(x => (x.path ?? '').replace(/\/+$/, '').endsWith('/' + d));
  const idle = async (ms = 120000) => wait(async () => { const st = await inv('scan_status', {}); return !st.running && !st.scanning; }, ms, 250);
  const addSource = async (p, d) => { await inv('add_source', { path: p }); const s = await wait(() => srcByDir(d), 15000); await sleep(800); await idle(); await sleep(600); return s; };
  const facesOf = async sid => ((await inv('query_faces', { q: mkq('', [sid]) })).items || []);
  const totalFaces = async () => ((await inv('list_sources', {})).reduce((a, s) => a + (s.faceCount || 0), 0));
  const pathOf = f => f.path ?? f.row?.path ?? null;
  const num = () => qa('[data-testid=task-num]').map(x => x.textContent.trim());
  const toasts = () => qa('[role=status],[data-testid=toast],.fa-toast').map(x => x.textContent.trim().slice(0, 100));
  // Quan sát khay: ghi mỗi lần trạng thái đổi (nút, các dòng, vòng). Tự mở popover để thấy dòng.
  const OB = { ev: [], on: false, last: '' };
  const state = () => ({ btn: !!q('[data-testid=task-button]'), label: q('[data-testid=task-button]')?.getAttribute('aria-label') ?? null, count: q('[data-testid=task-count]')?.textContent ?? null, ring: !!q('[data-testid=task-ring]'), rows: qa('[data-testid=task-row]').map(r => (r.getAttribute('data-kind') || '') + '|' + (r.getAttribute('data-state') || '') + '|' + r.textContent.trim().replace(/\s+/g, ' ').slice(0, 90)) });
  setInterval(() => { if (!OB.on) return; const b = q('[data-testid=task-button]'); if (b && !q('[data-testid=task-popover]') && OB.autoOpen !== false) b.click(); const s = state(); const k = JSON.stringify(s); if (k !== OB.last) { OB.last = k; OB.ev.push({ t: Date.now(), ...s }); } }, 30);
  const obStart = () => { OB.ev = []; OB.last = ''; OB.on = true; OB.autoOpen = true; return Date.now(); };
  const obStop = () => { OB.on = false; };
  const doneRe = /Xong|Done|✓/;
  const kindSeen = (k, since = 0) => OB.ev.filter(e => e.t >= since && e.rows.some(r => r.startsWith(k + '|')));
  const lingerOf = (kind, since) => { // từ lúc dòng kind báo xong tới lúc nút khay biến mất
    const done = OB.ev.find(e => e.t >= since && e.rows.some(r => r.startsWith(kind + '|') && doneRe.test(r)));
    if (!done) return { doneAt: null, goneAt: null, lingerMs: null };
    const gone = OB.ev.find(e => e.t > done.t && !e.btn);
    return { doneAt: done.t - since, goneAt: gone ? gone.t - since : null, lingerMs: gone ? gone.t - done.t : null };
  };
  const compact = since => OB.ev.filter(e => e.t >= since).slice(0, 14).map(e => ({ dt: e.t - since, btn: e.btn, count: e.count, rows: e.rows.map(x => x.slice(0, 70)) }));
  const closePop = async () => { document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); await sleep(250); };
  const waitNoTray = async (ms = 20000) => { OB.autoOpen = false; await closePop(); const r = await wait(() => !q('[data-testid=task-button]'), ms, 200); OB.autoOpen = true; return !!r; };

  await wait(() => window.__faLocale, 20000); window.__faLocale?.('vi'); await sleep(500);
  const phases = (window.__Q9_ONLY || '').split(',').filter(Boolean);
  const on = p => phases.length === 0 || phases.includes(p);
  const R = {};
  try {
    await wait(() => q('[data-testid=splash]') === null && (q('.fa-onb') || q('.fa-lib')), 90000);
    for (let i = 0; i < 20 && q('.fa-onb'); i++) { [...document.querySelectorAll('button')].find(x => /^(Bỏ qua|Skip|Để sau|Đã hiểu|Tiếp tục|Bắt đầu|Vào thư viện)/.test(x.textContent.trim()))?.click(); await sleep(500); }
    await sleep(800);
    await log('q9-boot', true, { tray: !!q('[data-testid=task-button]') });

    // đếm lượt quét qua TaskHub (KHÔNG gọi scan_subscribe: kênh đó chỉ có 1 người nghe, gọi sẽ cướp sự kiện của giao diện)
    const scanRuns = new Map();
    setInterval(async () => { try { const l = await inv('list_tasks', {}); (l.tasks || []).forEach(t => { if (t.kind === 'scan' && !scanRuns.has(t.id)) scanRuns.set(t.id, { t: Date.now(), trigger: t.trigger, title: t.titleArgs }); }); } catch (e) {} }, 120);
    const BL = { eps: [], cur: null, samples: 0 };
    setInterval(() => { BL.samples++; const cnt = Number((q('[data-testid=view-count]')?.textContent || '0').replace(/\D/g, '')); const blank = cnt > 0 && qa('[data-testid=face-name]').length === 0 && !q('[data-testid=library-grid-scroller] .fa-skel'); if (blank && !BL.cur) BL.cur = { s: Date.now(), cnt }; if (!blank && BL.cur) { BL.eps.push({ ms: Date.now() - BL.cur.s, cnt: BL.cur.cnt }); BL.cur = null; } }, 50);
    const scanFin = []; window.__scanFin = scanFin;
    if (phases.includes('sub')) { try { await inv('scan_subscribe', { onEvent: ch(e => { if (e.type === 'finished') scanFin.push({ t: Date.now(), changed: e.changed ?? null, trigger: e.trigger ?? null, stats: e.stats ? { analyzed: e.stats.analyzed, relinked: e.stats.relinked, seen: e.stats.seen, unchanged: e.stats.unchanged } : null }); }) }); } catch (e) {} }
    const scanEv = { get length() { return scanRuns.size; }, slice: n0 => [...scanRuns.values()].slice(n0) };

    if (on('probe')) {
      try {
        const grid = () => { const sc = q('[data-testid=library-grid-scroller]'); return { scroller: !!sc, scH: sc ? Math.round(sc.getBoundingClientRect().height) : null, cells: qa('.fa-cell').length, names: qa('[data-testid=face-name]').length, empty: /Chưa có font/.test(document.body.innerText), main: (q('main, [data-testid=library-main], .fa-main')?.innerHTML || '').slice(0, 260), sideAll: (qa('[data-testid=sidebar] a, [data-testid=sidebar] button').find(b => /^Tất cả/.test(b.textContent.trim()))?.textContent || '').trim(), allTxt: (q('.fa-lib-head, header')?.innerText || '').replace(/\s+/g, ' ').slice(0, 40), vis: document.visibilityState, focus: document.hasFocus(), w: innerWidth, h: innerHeight }; };
        await log('q9-p0-before', true, grid());
        const sa = await addSource('__Q__/fonts/act', 'act');
        for (const ms of [200, 1000, 3000]) { await sleep(ms); await log('q9-p1-after-add+' + ms, true, grid()); }
        await snap('p1-grid-after-add');
        const r = await inv('query_faces', { q: mkq('', []) }); await log('q9-p2-query', true, { items: (r.items || []).length, total: r.total ?? null, keys: Object.keys(r) });
        q('[aria-label="Tất cả"], [data-testid=nav-all]')?.click(); await sleep(1500); await log('q9-p3-after-click-all', true, grid());
        const sc = q('[data-testid=library-grid-scroller]'); if (sc) { sc.scrollTop = 5; } await sleep(500); await log('q9-p4-after-scroll', true, grid());
        const lbl = qa('button,[role=button],a').map(b => (b.getAttribute('aria-label') || b.textContent || '').trim()).filter(x => /vi|Việt|lọc|filter|Tất cả/i.test(x)).slice(0, 10); await log('q9-p5-filters', true, lbl);
      } catch (e) { await log('q9-p-error', false, E(e)); }
    }

    // ───── 1. kích hoạt / tắt / chuyển chỗ ─────
    if (on('act')) {
      try {
        const sa = await addSource('__Q__/fonts/act', 'act'); const sb = await addSource('__Q__/fonts/act2', 'act2');
        const fa = (await facesOf(sa.id)); R.actFaces = fa; R.act2Path = '__Q__/fonts/act2';
        await log('q9-1.0-setup', fa.length === 6, { faces: fa.length, act2: (await facesOf(sb.id)).length, sample: fa[0] && Object.keys(fa[0]).slice(0, 12) });
        await waitNoTray(15000);
        // bật
        let t = obStart(); const rep = await inv('activation_activate', { faces: fa.map(keyOf), scope: 'session', proceed: false, onProgress: quiet() });
        await sleep(400); const st = (await inv('activation_state', {})).filter(x => x.state === 'active');
        await log('q9-1.1-activate', (rep.changed ?? 0) >= 6 || st.length >= 6, { changed: rep.changed, failed: rep.failed, needsConfirmation: rep.needsConfirmation, active: st.length, paths: [...new Set(st.map(x => (x.activePath || '').replace(/\/[^/]+$/, '')))] });
        await go('act-on', 0);
        await sleep(1500); await snap('1a-activate-tray');
        await wait(() => !q('[data-testid=task-button]'), 20000, 200); obStop();
        const lg1 = lingerOf('activation', t);
        await log('q9-1.1b-tray-activate', kindSeen('activation', t).length > 0 && lg1.lingerMs !== null, { rowsSeen: kindSeen('activation', t).length, ...lg1, timeline: compact(t) });
        await sleep(2500); // chờ shell kiểm CoreText
        // tắt
        t = obStart(); const rep2 = await inv('activation_deactivate', { faces: fa.map(keyOf), force: false, onProgress: quiet() });
        await sleep(400); const st2 = (await inv('activation_state', {})).filter(x => x.state === 'active');
        await go('act-off', 0); await sleep(1500); await snap('1b-deactivate-tray');
        await wait(() => !q('[data-testid=task-button]'), 20000, 200); obStop();
        const lg2 = lingerOf('activation', t);
        await log('q9-1.2-deactivate', st2.length === 0 && kindSeen('activation', t).length > 0, { changed: rep2.changed, failed: rep2.failed, activeLeft: st2.length, ...lg2, timeline: compact(t) });
        await sleep(2500);
        // chuyển chỗ: bật từ act rồi chuyển sang act2
        await inv('activation_activate', { faces: fa.map(keyOf), scope: 'session', proceed: false, onProgress: quiet() }); await waitNoTray(15000);
        const before = (await inv('activation_state', {})).filter(x => x.state === 'active').map(x => ({ h: x.hashHex, p: x.activePath }));
        const curDir = (before[0]?.p || '').replace(/\/[^/]+$/, ''); const tgt = curDir.endsWith('/act2') ? '__Q__/fonts/act' : '__Q__/fonts/act2'; R.act2Path = tgt;
        t = obStart(); const out = await inv('activation_relocate', { faces: fa.map(keyOf), preferUnder: tgt, onProgress: quiet() });
        await sleep(400); const after = (await inv('activation_state', {})).filter(x => x.state === 'active');
        const under2 = after.filter(x => (x.activePath || '').startsWith(R.act2Path)).length;
        await go('act-reloc', 0); await sleep(1500); await snap('1c-relocate-tray');
        await wait(() => !q('[data-testid=task-button]'), 20000, 200); obStop();
        const lg3 = lingerOf('activation', t);
        await log('q9-1.3-relocate', under2 === before.length && under2 > 0 && kindSeen('activation', t).length > 0, { from: curDir.split('/').slice(-1)[0], to: tgt.split('/').slice(-1)[0], before: before.length, changed: out.changed, failed: out.failed, underAct2: under2, ...lg3, timeline: compact(t) });
        await sleep(2500);
        await inv('activation_deactivate_all', { onProgress: quiet() }); await waitNoTray(15000); await go('act-end', 2500);
      } catch (e) { await log('q9-1-error', false, E(e)); }
    }

    // ───── 2. kéo thả 50 file ─────
    if (on('imp')) {
      try {
        const c = await inv('create_case', { name: 'QA import', brief: null, settings: null });
        const paths = []; for (let i = 0; ; i++) { /* danh sách đường dẫn lấy từ shell qua nguồn tạm */ break; }
        const sdrop = await addSource('__Q__/fonts/drop50', 'drop50'); // dùng nguồn chỉ để lấy tên file; nhập thực bằng đường dẫn tới thư mục dropx (bản sao)
        const dfs = (await facesOf(sdrop.id)).map(pathOf).filter(Boolean);
        await inv('remove_source', { sourceId: sdrop.id }); await sleep(800); await idle(); await waitNoTray(15000);
        const files = dfs.map(p => p.replace('/fonts/drop50/', '/fonts/dropx/'));
        R.dropN = files.length;
        const samples = []; let sawCounting = false; const prog = [];
        let t = obStart();
        const pl = setInterval(() => { const x = qa('[data-testid=task-num]').map(e => e.textContent.trim()); if (x.length) { samples.push(x[0]); if (x.some(s => /Đang đếm|Counting/.test(s))) sawCounting = true; } }, 25);
        const res = await inv('import_paths_into', { paths: files, target: { kind: 'case', id: c.id }, onProgress: ch(m => prog.push([m.phase, m.done, m.total])) });
        await sleep(300); clearInterval(pl); await snap('2a-import-done');
        await wait(() => !q('[data-testid=task-button]'), 20000, 200); obStop();
        const nums = samples.map(s => { const m = s.match(/^([\d.]+)\/([\d.]+)$/); return m ? [Number(m[1].replace(/\./g, '')), Number(m[2].replace(/\./g, ''))] : null; }).filter(Boolean);
        let back = 0; for (let i = 1; i < nums.length; i++) if (nums[i][0] < nums[i - 1][0]) back++;
        let progBack = 0; const backAt = []; for (let i = 1; i < prog.length; i++) if (prog[i][0] === prog[i - 1][0] && prog[i][1] < prog[i - 1][1]) { progBack++; backAt.push([prog[i - 1], prog[i]]); }
        const lg = lingerOf('other', t);
        await log('q9-2-import-drop50', files.length === 50 && back === 0 && progBack === 0, { files: files.length, result: { added: (res.added || []).length, alreadyThere: res.alreadyThere, keys: Object.keys(res || {}), skipped: (res.skipped || []).length, cancelled: res.cancelled }, traySamples: samples.length, firstSamples: samples.slice(0, 5), lastSamples: samples.slice(-3), sawCounting, backSteps: back, progressEvents: prog.length, progressBack: progBack, backAt: backAt.slice(0, 4), phases: [...new Set(prog.map(p => p[0]))], progFirst: prog.slice(0, 3), progLast: prog.slice(-3), ...lg, timeline: compact(t).slice(0, 8) });

        // thả THƯ MỤC (cần đếm file trước)
        try {
          const c2 = await inv('create_case', { name: 'QA import dir', brief: null, settings: null });
          const samples2 = []; let counting2 = false; const prog2 = []; const t2 = obStart();
          const pl2 = setInterval(() => { const x = qa('[data-testid=task-num]').map(e => e.textContent.trim()); if (x.length) { samples2.push(x[0]); if (x.some(s => /Đang đếm|Counting/.test(s))) counting2 = true; } }, 25);
          const res2 = await inv('import_paths_into', { paths: ['__Q__/fonts/dropdirx'], target: { kind: 'case', id: c2.id }, onProgress: ch(m => prog2.push([m.phase, m.done, m.total])) });
          await sleep(300); clearInterval(pl2); await snap('2b-import-dir-done'); await wait(() => !q('[data-testid=task-button]'), 20000, 200); obStop();
          const nums2 = samples2.map(s => { const m = s.match(/^([\d.]+)\/([\d.]+)$/); return m ? [Number(m[1].replace(/\./g, '')), Number(m[2].replace(/\./g, ''))] : null; }).filter(Boolean);
          let back2 = 0; for (let i = 1; i < nums2.length; i++) if (nums2[i][0] < nums2[i - 1][0]) back2++;
          let pb2 = 0; const bk2 = []; for (let i = 1; i < prog2.length; i++) if (prog2[i][0] === prog2[i - 1][0] && prog2[i][1] < prog2[i - 1][1]) { pb2++; bk2.push([prog2[i - 1], prog2[i]]); }
          await log('q9-2c-import-dir', counting2 && back2 === 0, { added: (res2.added || []).length, sawCounting: counting2, trayBackSteps: back2, samples: samples2.length, first: samples2.slice(0, 6), last: samples2.slice(-2), progressBack: pb2, backAt: bk2.slice(0, 4), phases: [...new Set(prog2.map(p => p[0]))], progFirst: prog2.slice(0, 3), progLast: prog2.slice(-2) });
        } catch (e) { await log('q9-2c-error', false, E(e)); }
        const cs = await inv('get_case', { id: c.id }).catch(() => null);
        await log('q9-2b-case-has-faces', true, { case: cs && (cs.faceCount ?? cs.faces?.length ?? Object.keys(cs).slice(0, 8)) });
      } catch (e) { await log('q9-2-error', false, E(e)); }
    }

    // ───── 3. tải từ cloud mô phỏng ─────
    if (on('cloud')) {
      try {
        const sc = await addSource('__Q__/sim/Library/CloudStorage/cloud', 'cloud'); R.cloud = sc;
        await sleep(1500); await waitNoTray(30000);
        const n0 = phases.includes('sub') ? scanFin.length : scanEv.length; const faces0 = (await facesOf(sc.id)).length;
        let t = obStart(); const kinds = new Set(); const labels = new Set(); const cp = setInterval(() => { qa('[data-testid=task-row]').forEach(r => { kinds.add(r.getAttribute('data-kind')); labels.add(r.textContent.trim().replace(/\s+/g, ' ').slice(0, 80)); }); }, 40);
        await go('cloud-start', 0); R.cloudT = Date.now();
        await sleep(30000); await snap('3a-cloud-mid');
        try { await inv('source_download_cloud', { sourceId: sc.id, onProgress: quiet() }).then(r => log('q9-3b-source-download', true, r), e => log('q9-3b-source-download', false, E(e))); } catch (e) { await log('q9-3b-source-download', false, E(e)); }
        await sleep(2000); await snap('3a2-after-download-call');
        await sleep(30000); // chờ nguồn yên (quiet 8 s) rồi quét lần cuối
        await sleep(15000); clearInterval(cp);
        await snap('3b-cloud-after');
        const sc2 = (await inv('list_sources', {})).find(x => x.id === sc.id);
        const scansDuring = phases.includes('sub') ? scanFin.slice(n0).map(x => ({ t: x.t, changed: x.changed + '/' + x.trigger + '/an' + (x.stats && x.stats.analyzed) + '/seen' + (x.stats && x.stats.seen) })) : scanEv.slice(n0).map(x => ({ t: x.t, changed: x.trigger })); const lastWrite = R.cloudT + 48000;
        const afterQuiet = scansDuring.filter(e => e.t > lastWrite + 1000);
        await log('q9-3-cloud-sim', true, { facesBefore: faces0, facesAfter: sc2 && sc2.faceCount, scansTotal: scansDuring.length, scansAfterWritesEnd: afterQuiet.length, scanTimes: scansDuring.map(e => Math.round((e.t - R.cloudT) / 1000)), triggers: scansDuring.map(e => e.changed), kinds: [...kinds], labels: [...labels].slice(0, 8), cloudDownloadRow: [...kinds].includes('cloudDownload') });
        obStop(); await waitNoTray(20000);
      } catch (e) { await log('q9-3-error', false, E(e)); }
    }

    // ───── 4. huỷ giữa chừng ─────
    if (on('cancel')) {
      try { // 4a. huỷ quét
        const fb0 = await totalFaces();
        obStart(); await inv('add_source', { path: '__Q__/fonts/big' }).catch(() => {});
        await wait(() => qa('[data-testid=task-num]').some(x => /^\d/.test(x.textContent) && Number(x.textContent.trim().split('/')[0].replace(/\./g, '')) >= 200), 60000, 80);
        const numBefore = num(); const x = q('[data-testid=task-cancel]');
        x?.click(); await sleep(250); const conf = q('[data-testid=task-confirm]'); const confText = conf?.textContent ?? null;
        await snap('4a-scan-cancel-confirm');
        q('[data-testid=task-keep]')?.click(); await sleep(250);
        const stillRunning = !!q('[data-testid=task-row]') && !q('[data-testid=task-confirm]');
        q('[data-testid=task-cancel]')?.click(); await sleep(200); q('[data-testid=task-confirm-yes]')?.click(); await sleep(400);
        await snap('4a-scan-cancelling');
        await wait(() => !q('[data-testid=task-button]'), 30000, 200); obStop();
        await sleep(1500);
        const srcBig = await srcByDir('big'); const bigFaces = srcBig ? srcBig.faceCount : null;
        const st = await inv('scan_status', {});
        await log('q9-4a-scan-cancel', bigFaces !== null && bigFaces > 0 && bigFaces < 1150 && !st.running, { numAtCancel: numBefore, hadConfirm: !!conf, confirmText: confText, keepKeptRunning: stillRunning, bigFacesAfter: bigFaces, of: 1150, scanRunning: st.running, scanning: st.scanning });
        R.bigId = srcBig?.id;
        // quét lại phần còn lại: lên đủ
        if (srcBig) { await inv('rescan_sources', { sourceIds: [srcBig.id] }); await sleep(1500); await idle(180000); const fc = async () => ((await inv('list_sources', {})).find(x => x.id === srcBig.id) || {}).faceCount; await wait(async () => (await fc()) === 1150, 120000, 1000); const f2 = await fc(); await log('q9-4a2-rescan-completes', f2 === 1150, { facesAfterRescan: f2 }); await waitNoTray(20000); }
      } catch (e) { await log('q9-4a-error', false, E(e)); }
      try { // 4b. huỷ xử lý font trùng + Undo
        const sd = await addSource('__Q__/fonts/dupsx', 'dupsx'); await waitNoTray(20000);
        const groups = await inv('duplicate_groups', { q: {} });
        const items = groups.filter(g => g.faces.length > 1 && g.faces.every(f => pathOf(f))).map(g => ({ kind: g.kind, key: g.key, keeperPath: pathOf(g.faces[0]), loserPaths: g.faces.slice(1).map(pathOf), memberPaths: g.faces.map(pathOf) }));
        const losers = items.reduce((a, i) => a + i.loserPaths.length, 0);
        await log('q9-4b0-dup-groups', items.length >= 5, { groups: groups.length, items: items.length, losers, kinds: [...new Set(groups.map(g => g.kind))] });
        const fileSnap = async () => 0;
        obStart(); const p = inv('resolve_apply', { req: { items, action: 'hide', explicitTrashPaths: [], mode: 'chosen' } }).then(r => ({ ok: true, r }), e => ({ ok: false, e: E(e) }));
        await wait(() => q('[data-testid=task-cancel]'), 15000, 20);
        const rowsNow = state().rows; q('[data-testid=task-cancel]')?.click(); await sleep(120);
        const conf = q('[data-testid=task-confirm]'); const confText = conf?.textContent ?? null; await snap('4b-dup-confirm');
        q('[data-testid=task-keep]')?.click(); await sleep(200);
        const keptRunning = !q('[data-testid=task-confirm]') && !!q('[data-testid=task-row]');
        q('[data-testid=task-cancel]')?.click(); await sleep(150); q('[data-testid=task-confirm-yes]')?.click(); await sleep(300);
        const res = await p; await snap('4b-dup-after'); obStop();
        const rr = res.r || {}; const doneN = (rr.done || []).length;
        const g2 = await inv('duplicate_groups', { q: {} });
        await log('q9-4b-dup-cancel', res.ok && hasOwn(rr, 'done'), { rowsAtCancel: rowsNow, hadConfirm: !!conf, confirmText: confText, keepKeptRunning: keptRunning, resultKeys: Object.keys(rr), cancelled: rr.cancelled ?? null, done: doneN, failed: (rr.failed || []).length, losersPlanned: losers, groupsBefore: items.length, groupsAfter: g2.length });
        R.dupBatch = rr.batchId; R.dupDone = (rr.done || []).map(d => d.id);
        // Undo
        if (R.dupDone.length) { const u = await inv('resolve_undo', { ids: R.dupDone }); const g3 = await inv('duplicate_groups', { q: {} }); await log('q9-4b2-dup-undo', g3.length >= items.length - 1, { undone: (u.done || []).length, failed: (u.failed || []).length, needsRescan: u.needsRescan, groupsAfterUndo: g3.length, groupsBefore: items.length }); }
        else await log('q9-4b2-dup-undo', false, 'không có quyết định nào để hoàn tác (huỷ quá sớm / xong quá nhanh)');
        await waitNoTray(20000);
      } catch (e) { await log('q9-4b-error', false, E(e)); }
      try { // 4c. huỷ chép vào kho (Organized)
        await inv('organized_create', { path: '__Q__/OrgStore' });
        const sbig = R.bigId; await go('org-before', 1500);
        obStart();
        const prog = []; const op = inv('organized_import', { scope: { sourceIds: sbig ? [sbig] : null, downloadFirst: false }, onProgress: ch(m => prog.push(m.done)) }).then(r => ({ ok: true, r }), e => ({ ok: false, e: E(e) }));
        await wait(() => q('[data-testid=task-cancel]'), 20000, 20); await sleep(1200);
        q('[data-testid=task-cancel]')?.click(); await sleep(150); const conf = q('[data-testid=task-confirm]'); const confText = conf?.textContent ?? null; await snap('4c-org-confirm');
        q('[data-testid=task-confirm-yes]')?.click(); await sleep(300);
        const res = await op; obStop(); const rr = res.r || {};
        await log('q9-4c-org-cancel', res.ok && rr.cancelled === true, { hadConfirm: !!conf, confirmText: confText, cancelled: rr.cancelled, copied: rr.copied, issueCount: rr.issueCount, progressEvents: prog.length, err: res.ok ? null : res.e });
        await go('org-cancelled', 7000); await waitNoTray(20000);
      } catch (e) { await log('q9-4c-error', false, E(e)); }
    }


    if (on('ctl')) {
      try {
        const s1 = await addSource('__Q__/fonts/big', 'big'); await wait(async () => (await facesOf(s1.id)).length >= 1000, 120000, 1000); await sleep(3000); await idle(120000);
        const f = (await facesOf(s1.id)).length; const lst = (await inv('list_sources', {})).find(x => x.id === s1.id);
        const hashes = new Set((await facesOf(s1.id)).map(x => x.hashHex)).size;
        await log('q9-ctl-big-full-scan', f === 1150, { faces: f, uniqueHashes: hashes, sourceFaceCount: lst && lst.faceCount, of: 1150 });
      } catch (e) { await log('q9-ctl-error', false, E(e)); }
    }

    if (on('resort')) {
      try {
        const sl = await addSource('__Q__/fonts/lib', 'lib'); const fl = await facesOf(sl.id);
        await inv('organized_create', { path: '__Q__/OrgR' });
        const imp = await inv('organized_import', { scope: { sourceIds: [sl.id], downloadFirst: false }, onProgress: quiet() });
        await waitNoTray(30000);
        await log('q9-6.0-org-copy', imp.copied === fl.length, { faces: fl.length, copied: imp.copied, issues: imp.issueCount });
        await go('resort-before', 2500);
        const ov = fl.slice(0, 200); for (const f of ov) await inv('set_classification_override', { over: { face: keyOf(f), class: 'serif', subclass: null } });
        const pv = await inv('organized_resort_preview', {});
        await log('q9-6.1-preview', pv.total > 0, { total: pv.total, sample: pv.moves.slice(0, 2).map(m => [m.from, m.to]) });
        // áp dụng: quan sát khay + thời gian
        let t = obStart(); const t1 = Date.now(); const ap = await inv('organized_resort_apply', {}); const ms = Date.now() - t1; await sleep(500);
        const rowsSeen = OB.ev.filter(e => e.t >= t && e.rows.length).length; const btnSeen = OB.ev.filter(e => e.t >= t && e.btn).length; obStop();
        await log('q9-6.2-apply', ap.moved === pv.total && ap.failed === 0, { moved: ap.moved, failed: ap.failed, planned: pv.total, ms, trayRowsSeen: rowsSeen, trayButtonSeen: btnSeen });
        await go('resort-applied', 2500);
        const st = await inv('activation_state', {}).catch(() => []);
        const pv2 = await inv('organized_resort_preview', {}); await log('q9-6.3-after-apply-preview', pv2.total === 0, { total: pv2.total });
        const und = await inv('organized_resort_undo', { batchId: ap.batchId }); await log('q9-6.4-undo', und.moved === ap.moved && und.failed === 0, { moved: und.moved, failed: und.failed });
        await go('resort-undone', 2500);
        // huỷ giữa chừng: bắn organized_cancel ngay khi bắt đầu áp dụng lần 2
        t = obStart(); const pa = inv('organized_resort_apply', {}).then(r => ({ ok: true, r }), e => ({ ok: false, e: E(e) })); const cancelRes = await inv('organized_cancel', {}).catch(e => E(e)); const ra = await pa; obStop();
        await log('q9-6.5-cancel-during-apply', true, { cancelReturned: cancelRes, apply: ra.ok ? { moved: ra.r.moved, failed: ra.r.failed } : ra.e, plannedTotal: pv.total, trayRows: OB.ev.filter(e => e.t >= t && e.rows.length).length });
        await go('resort-cancelled', 2500);
        if (ra.ok) { const u2 = await inv('organized_resort_undo', { batchId: ra.r.batchId }); await log('q9-6.6-undo-again', u2.failed === 0, { moved: u2.moved, failed: u2.failed }); await go('resort-final', 2500); }
      } catch (e) { await log('q9-6-error', false, E(e)); }
    }

    // ───── 5. sidebar thu gọn + giao diện tối với ≥3 việc ─────
    if (on('ui')) {
      try {
        obStart();
        await inv('add_source', { path: '__Q__/fonts/ui1' }).catch(() => {}); await inv('add_source', { path: '__Q__/fonts/ui2' }).catch(() => {}); await inv('add_source', { path: '__Q__/fonts/ui3' }).catch(() => {}); await inv('add_source', { path: '__Q__/fonts/ui4' }).catch(() => {});
        await wait(() => (state().rows.length >= 3) || (q('[data-testid=task-count]')?.textContent || '') >= '3', 15000, 40);
        const s0 = state(); await snap('5a-light-expanded');
        const dark = q('[aria-label="Giao diện sáng/tối"]'); dark?.click(); await sleep(500);
        await snap('5b-dark-expanded');
        q('[aria-label="Thu gọn thanh bên"]')?.click(); await sleep(700);
        const collapsed = !!q('.fa-side[data-collapsed="true"]'); const s1 = state();
        await snap('5c-dark-collapsed');
        q('[data-testid=task-button]')?.click(); await wait(() => q('[data-testid=task-popover]'), 3000); await sleep(400);
        const s2 = state(); await snap('5d-dark-collapsed-popover');
        const pop = q('[data-testid=task-popover]'); const rect = pop ? pop.getBoundingClientRect() : null;
        await log('q9-5-ui', collapsed && s2.rows.length > 0, { rowsExpanded: s0.rows.length, countLabel: s0.count, label: s0.label, collapsed, btnCollapsed: s1.btn, rowsCollapsed: s2.rows.length, rowsText: s2.rows, popRect: rect && { x: Math.round(rect.x), y: Math.round(rect.y), w: Math.round(rect.width), h: Math.round(rect.height), inView: rect.x >= 0 && rect.right <= innerWidth && rect.bottom <= innerHeight } });
        await wait(() => !q('[data-testid=task-ring]'), 120000, 300); await sleep(500); await snap('5e-dark-collapsed-done');
        await closePop(); q('[aria-label="Mở rộng thanh bên"]')?.click(); q('[aria-label="Giao diện sáng/tối"]')?.click(); await sleep(400); obStop(); await waitNoTray(20000);
      } catch (e) { await log('q9-5-error', false, E(e)); }
    }
    await log('q9-blank-grid', BL.eps.length === 0, { samples: BL.samples, episodes: BL.eps.slice(0, 12), n: BL.eps.length, maxMs: Math.max(0, ...BL.eps.map(e => e.ms)) });
    await log('q9-done', true, null);
  } catch (e) { await log('q9-error', false, E(e)); }
  function hasOwn(o, k) { return o != null && Object.prototype.hasOwnProperty.call(o, k); }
})();
