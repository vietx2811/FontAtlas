// Tự kiểm 0.5.0-beta.2 (07 QA/DevOps). Chạy trong cửa sổ app DEBUG (FONTATLAS_QA_SCRIPT) với FONTATLAS_DATA_DIR tạm trên VXData.
// Chỉ dùng font thử chép từ fixture của repo. __Q__ được run07.sh thay bằng thư mục thử. Kết quả ghi vào bảng settings (ui.qa.NNN).
(async () => {
  const T = window.__TAURI_INTERNALS__;
  const inv = (c, a) => T.invoke(c, a);
  const T0 = Date.now(); let n = 0;
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const wait = async (fn, ms = 20000) => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) {} await sleep(150); } return null; };
  const log = async (step, ok, detail) => { n++; try { await inv('set_setting', { key: 'ui.qa.' + String(n).padStart(3, '0'), value: JSON.stringify({ step, ok, detail, t: Math.round((Date.now() - T0) / 1000) }) }); } catch (e) {} };
  const ch = cb => '__CHANNEL__:' + T.transformCallback(x => { try { cb(x && x.message !== undefined ? x.message : x); } catch (e) {} });
  const quiet = () => '__CHANNEL__:' + T.transformCallback(() => {});
  const keyOf = r => ({ hashHex: r.hashHex ?? r.hash_hex, faceIndex: r.faceIndex ?? r.face_index ?? 0 });
  const mkq = (text, sourceIds, extra = {}) => ({ text, vi: null, license: [], variable: null, italic: null, sourceIds, favoritesOnly: false, collectionId: null, tagIds: [], includeMissing: false, sort: 'family', descending: false, offset: 0, limit: 200, axisFilter: [], ...extra });
  const E = e => { try { return (e && e.message ? e.message + ' | ' : '') + ((e && e.stack) ? e.stack : JSON.stringify(e)); } catch (x) { return String(e); } };
  const errs = []; const oe = console.error.bind(console); console.error = (...a) => { try { errs.push(a.map(x => String(x && x.stack || x)).join(' ').slice(0, 300)); } catch (e) {} oe(...a); };
  const textOf = el => (el ? el.innerText.replace(/\n{2,}/g, '\n').slice(0, 1800) : null);
  // Báo điểm dừng cho run07.sh (nó làm việc ngoài app: kiểm đĩa, mở/thoát app thử). Chiều script→shell qua bảng settings; không dùng chiều ngược lại
  // (app không thấy ngay giá trị settings do tiến trình khác ghi), nên chỉ ghi mốc rồi chờ KẾT QUẢ quan sát được.
  const go = async (name, settle = 0) => { await log('q7-wait-' + name, true, null); const at = Date.now(); if (settle) await sleep(settle); return at; };
  const srcByDir = async d => (await inv('list_sources', {})).find(x => (x.path ?? '').replace(/\/+$/, '').endsWith('/' + d));
  const addSource = async d => { await inv('add_source', { path: '__Q__/fonts/' + d }); const s = await wait(() => srcByDir(d), 15000); await wait(async () => { const st = await inv('scan_status', {}); return !st.running && !st.scanning; }, 30000); await sleep(800); return s; };
  const facesOf = async sid => ((await inv('query_faces', { q: mkq('', [sid]) })).items || []);
  const closeDlg = async () => { document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); await sleep(300); };
  const openSettings = async () => { document.querySelector('button[aria-label="Cài đặt"], button[aria-label="Settings"]')?.click(); await wait(() => document.querySelector('.fa-set-nav button'), 8000); };
  const navTo = async re => { [...document.querySelectorAll('.fa-set-nav button')].find(b => re.test(b.textContent))?.click(); await sleep(400); };
  const btnText = (re, root = document) => [...root.querySelectorAll('button')].find(b => re.test(b.textContent.trim()));
  const dlgs = () => [...document.querySelectorAll('[role=dialog]')];
  const dlg = () => dlgs().pop() || null; // hộp thoại trong cùng (Cài đặt cũng là role=dialog)
  // i18n thiếu: chuỗi trông như khoá thô (vd. org.st.root) hoặc chuỗi đứng nguyên dạng khoá.
  const rawKeys = txt => [...new Set((txt || '').match(/\b[a-z][a-zA-Z]+(?:\.[a-zA-Z]+){1,4}\b/g) || [])].filter(k => !/\.(com|org|app|ttf|otf|md|png|json)$/.test(k)).slice(0, 12);
  await wait(() => window.__faLocale, 20000); window.__faLocale?.('vi'); await sleep(500);
  const phases = (window.__Q7_ONLY || '').split(',').filter(Boolean);
  const on = p => phases.length === 0 || phases.includes(p);
  const R = {}; // trạng thái dùng chung giữa các pha

  // Thoát onboarding (dữ liệu mới): bấm lần lượt các nút giống tests/qa/main_flows.js.
  try {
    const order = ['Đã hiểu', 'Không dùng', 'Bỏ qua', 'Để sau', 'Tiếp tục', 'Bắt đầu', 'Vào thư viện'];
    for (let i = 0; i < 40; i++) {
      await sleep(500);
      if (document.querySelector('[role=dialog] .fa-tips-grid')) { await closeDlg(); continue; }
      if (!document.querySelector('.fa-onb')) { if (document.querySelector('.fa-splash, [data-testid=splash]')) continue; break; }
      const b = order.map(t => btnText(new RegExp('^' + t))).find(Boolean); b?.click();
    }
    await wait(() => !document.querySelector('.fa-onb'), 15000);
    await log('q7-onboarding-left', !document.querySelector('.fa-onb'), null);
  } catch (e) { await log('q7-onboarding-left', false, E(e).slice(0, 300)); }

  // ───────── P0: nguồn font thử ─────────
  try {
    const s = await addSource('small'); R.small = s;
    const fs = s ? await facesOf(s.id) : [];
    R.smallFaces = fs; await log('q7-p0-source-small', !!s && fs.length >= 12, { faces: fs.length, expected: 12 });
  } catch (e) { await log('q7-p0-source-small', false, E(e).slice(0, 400)); }

  // ───────── 1. Organized Library ─────────
  if (on('org')) {
    const root = '__Q__/OrgStore';
    try { // 1.1 tạo kho trên VXData
      const chk = await inv('organized_check_location', { path: root });
      const st0 = await inv('organized_status', {});
      let created = null; if (!st0.configured && !chk.problem) created = await inv('organized_create', { path: root });
      const st1 = await inv('organized_status', {});
      await log('q7-1.1-create-store', !chk.problem && chk.writable && !chk.isCloud && chk.freeBytes > 0 && st1.configured, { check: chk, configured: st1.configured, libraryDir: st1.libraryDir, freeBytes: st1.freeBytes, isCloud: st1.isCloud });
      R.lib = st1.libraryDir;
    } catch (e) { await log('q7-1.1-create-store', false, E(e).slice(0, 400)); }
    try { // 1.2 xem trước chép: API + hộp thoại
      const plan = await inv('organized_plan', { scope: { sourceIds: [R.small.id], downloadFirst: false } });
      await openSettings(); await navTo(/(Thư viện|Library)/);
      const panelText = textOf(document.querySelector('[data-testid=organized-panel]'));
      document.querySelector('[data-testid=org-open-import]')?.click();
      const d = await wait(() => dlgs().length > 1, 8000); await sleep(1200);
      const txt = textOf(dlg());
      await log('q7-1.2-copy-preview', !!d && plan.toCopy === 12, { plan: { toCopy: plan.toCopy, bytes: plan.bytes, alreadyThere: plan.alreadyThere, cloudNotDownloaded: plan.cloudNotDownloaded, needsDownload: plan.needsDownload, enoughSpace: plan.enoughSpace }, dialogText: txt, panelText, rawKeys: rawKeys(txt + ' ' + panelText) });
      R.dialogImport = txt;
      await closeDlg();
    } catch (e) { await log('q7-1.2-copy-preview', false, E(e).slice(0, 400)); }
    try { // trước khi chép: bật 3 font từ nguồn gốc (để thử bước 5 "chuyển sang bản trong kho")
      R.pre = R.smallFaces.filter(f => !f.isVariable).slice(0, 3);
      const rep = await inv('activation_activate', { faces: R.pre.map(keyOf), scope: null, proceed: false, onProgress: quiet() });
      const act = (await inv('activation_state', {})).filter(x => x.state === 'active');
      R.relocBefore = act.map(x => ({ h: x.hashHex, p: x.activePath }));
      await log('q7-1.4-preactivate-from-source', act.length >= 2, { requested: R.pre.length, changed: rep.changed, needsConfirmation: rep.needsConfirmation, failed: rep.failed, activePaths: act.map(x => x.activePath) });
    } catch (e) { await log('q7-1.4-preactivate-from-source', false, E(e).slice(0, 400)); }
    try { // 1.3 chép có tiến độ + Huỷ
      const prog = []; const t0 = Date.now();
      const res = await inv('organized_import', { scope: { sourceIds: [R.small.id], downloadFirst: true }, onProgress: ch(m => prog.push({ ph: m.phase, d: m.done, tot: m.total })) });
      const st = await inv('organized_status', {});
      await log('q7-1.3a-copy', res.copied === 12 && !res.cancelled && res.issueCount === 0 && st.files === 12, { copied: res.copied, bytes: res.bytes, issues: res.issueCount, issueGroups: res.issueGroups, snapshot: res.snapshot, progressEvents: prog.length, phases: [...new Set(prog.map(p => p.ph))], last: prog[prog.length - 1], files: st.files, seconds: ((Date.now() - t0) / 1000).toFixed(1) });
      const b = await addSource('bulk'); R.bulk = b; // 100 file ~16 MB ⇒ đủ lâu để Huỷ giữa chừng
      await wait(async () => (await facesOf(b.id)).length >= 100, 60000);
      const planB = await inv('organized_plan', { scope: { sourceIds: [b.id], downloadFirst: false } });
      const before = (await inv('organized_status', {})).files;
      let first = false; const prog2 = [];
      const p = inv('organized_import', { scope: { sourceIds: [b.id], downloadFirst: false }, onProgress: ch(m => { prog2.push(m.done); if (!first) { first = true; inv('organized_cancel', {}); } }) });
      const res2 = await p; const after = (await inv('organized_status', {})).files;
      await log('q7-1.3b-cancel', res2.cancelled === true && res2.copied < 100 && after === before + res2.copied, { planCopy: planB.toCopy, planBytes: planB.bytes, cancelled: res2.cancelled, copied: res2.copied, filesBefore: before, filesAfter: after, progressEvents: prog2.length, snapshot: res2.snapshot });
    } catch (e) { await log('q7-1.3-copy', false, E(e).slice(0, 400)); }
    await go('org-copied', 7000); // run07.sh kiểm cấu trúc trên đĩa + checksum nguồn
    try { // 1.5 chuyển font đang bật sang bản trong kho + Hoàn tác
      const fs = R.pre; const before = R.relocBefore; const rep = { changed: before.length };
      document.querySelector('[data-testid=org-open-relocate]')?.click(); const d = await wait(() => dlgs().length > 1, 8000); await sleep(1000);
      const dtext = textOf(dlg()); await closeDlg();
      const out = await inv('activation_relocate', { faces: fs.map(keyOf), preferUnder: R.lib, onProgress: quiet() });
      const afterSt = (await inv('activation_state', {})).filter(x => x.state === 'active');
      const underLib = afterSt.filter(x => (x.activePath || '').startsWith(R.lib)).length;
      await log('q7-1.5a-relocate', rep.changed >= 2 && underLib === before.length && out.changed === before.length, { activated: rep.changed, relocateChanged: out.changed, failed: out.failed, activeUnderLibrary: underLib, active: afterSt.length, dialogText: dtext });
      const targets = before.map(b => { const f = fs.find(x => x.hashHex === b.h); return f ? { face: keyOf(f), path: b.p } : null; }).filter(Boolean);
      const und = await inv('activation_relocate_to', { targets, onProgress: quiet() });
      const back = (await inv('activation_state', {})).filter(x => x.state === 'active');
      const sameAsBefore = before.every(b => back.find(x => x.hashHex === b.h && x.activePath === b.p));
      await log('q7-1.5b-relocate-undo', sameAsBefore, { undoChanged: und.changed, failed: und.failed, backPaths: back.map(x => x.activePath).slice(0, 3) });
      await inv('activation_deactivate_all', { onProgress: quiet() });
    } catch (e) { await log('q7-1.5-relocate', false, E(e).slice(0, 400)); }
    try { // 1.6 sắp xếp lại: chỉ xem trước rồi Huỷ
      const pv = await inv('organized_resort_preview', {});
      document.querySelector('[data-testid=org-open-resort]')?.click(); await wait(() => dlgs().length > 1, 8000); await sleep(1200);
      const dtext = textOf(dlg()); const cancelBtn = btnText(/^(Huỷ|Hủy|Cancel|Đóng|Close)/, dlg() || document); cancelBtn?.click(); await sleep(400);
      const stillOpen = dlgs().length > 1;
      await log('q7-1.6-resort-preview-cancel', !!dtext, { total: pv.total, sampleMoves: (pv.moves || []).slice(0, 3), dialogText: dtext, cancelButton: cancelBtn ? cancelBtn.textContent.trim() : null, dialogClosed: !stillOpen });
    } catch (e) { await log('q7-1.6-resort', false, E(e).slice(0, 400)); }
    try { // 1.7 gói khôi phục
      const b = await inv('organized_status', {}); const path = await inv('organized_snapshot_now', {}); const a = await inv('organized_status', {});
      await log('q7-1.7-snapshot', a.snapshots === b.snapshots + 1, { snapshotsBefore: b.snapshots, snapshotsAfter: a.snapshots, last: a.lastSnapshot, returned: path });
    } catch (e) { await log('q7-1.7-snapshot', false, E(e).slice(0, 400)); }
    await go('org-done', 7000);
  }
  // ───────── 1.3 (chi tiết): Huỷ giữa chừng, có mốc thời gian ─────────
  if (on('cancel')) {
    try {
      const root = '__Q__/OrgStore'; const st0 = await inv('organized_status', {}); if (!st0.configured) await inv('organized_create', { path: root });
      const b = await addSource('bulk'); await wait(async () => (await facesOf(b.id)).length >= 100, 90000);
      const t0 = Date.now(); const ev = []; let cancelAt = null, cancelDone = null;
      const p = inv('organized_import', { scope: { sourceIds: [b.id], downloadFirst: false }, onProgress: ch(m => ev.push({ t: Date.now() - t0, d: m.done, tot: m.total, ph: m.phase })) });
      await sleep(4000); cancelAt = Date.now() - t0; const filesAtCancel = (await inv('organized_status', {})).files;
      const c = await inv('organized_cancel', {}); cancelDone = Date.now() - t0;
      const res = await p; const total = Date.now() - t0; const after = (await inv('organized_status', {})).files;
      await log('q7-1.3c-cancel-timed', res.cancelled === true && res.copied < 100 && after === res.copied, { cancelReturned: c, cancelAtMs: cancelAt, cancelDoneMs: cancelDone, totalMs: total, filesAtCancel, copied: res.copied, filesAfter: after, firstEvents: ev.slice(0, 3), lastEvents: ev.slice(-3), events: ev.length, snapshot: res.snapshot });
    } catch (e) { await log('q7-1.3c-cancel-timed', false, E(e).slice(0, 400)); }
    await go('cancel-done', 7000);
  }
  // ───────── 2/4. App Font Sets + màn "Đang bật" ─────────
  if (on('appset')) {
    try {
      const k = f => keyOf(f);
      // Font trùng PostScript với font ĐÃ CÀI trên máy chủ sẽ bị "xung đột, không bật": chỉ dùng font preflight sạch.
      const clean = []; const skipped = [];
      for (const f of R.smallFaces) { if (clean.length >= 2) break; const pf = await inv('activation_preflight', { faces: [k(f)] }); if (pf.needsConfirmation === 0 && pf.failed === 0) clean.push(f); else skipped.push(f.family); }
      if (clean.length < 2) throw new Error('không đủ 2 font thử không xung đột; bỏ qua: ' + skipped.join(','));
      const [fA, fB] = clean; const fC = fB; // chỉ 2 font fixture không trùng font cài trên máy chủ R.testFonts = { A: fA.family, B: fB.family, C: fC.family, skippedAsConflict: skipped };
      const mkCase = async (name, faces) => { const c = await inv('create_case', { name, brief: null, settings: null }); await inv('add_to_case', { caseId: c.id, faces: faces.map(k), origin: null }); return c; };
      // Case ↔ collection: giao diện (Sidebar/AppPicker) truyền ID CỦA CASE vào case_set_apps(collectionId), còn backend kiểm bảng collections.
      const c1 = await mkCase('QA07 Case A', [fA, fB, fC]);
      let caseErr = null; try { await inv('case_set_apps', { collectionId: c1.id, apps: [{ bundleId: 'com.apple.calculator', name: 'Calculator' }] }); } catch (e) { caseErr = E(e); }
      await log('q7-2.0-case-id-as-collection-id', caseErr === null, { caseId: c1.id, error: caseErr, note: 'Case mới tạo, chưa có collection nào: lưu app cho Case phải thành công' });
      // phần còn lại thử bằng COLLECTION thật (backend gắn app với collection): col1 = {B,C,A}, col2 = {A}
      const mkCol = async (name, faces) => { const c = await inv('create_collection', { name, color: null }); await inv('add_to_collection', { id: c.id, faces: faces.map(k) }); return c; };
      const cc1 = await mkCol('QA07 Set A', [fA, fB]), cc2 = await mkCol('QA07 Set B', [fA]);
      const col1 = cc1.id, col2 = cc2.id; R.c1 = c1; R.col = { col1, col2 };
      const apps = await inv('installed_apps', {});
      const REAL_BROWSERS = /^(Safari|Google Chrome|Firefox|Microsoft Edge|Arc|Brave Browser|Opera|Vivaldi)$/;
      const UI_BROWSERS = /^(com\.apple\.safari|com\.google\.chrome|org\.mozilla\.firefox|com\.microsoft\.edgemac|company\.thebrowser\.browser|com\.brave\.browser|com\.operasoftware\.opera|com\.vivaldi\.vivaldi)/i; // đúng regex của AppPicker.tsx
      const shownApps = apps.filter(a => !UI_BROWSERS.test(a.bundleId));
      const browsers = shownApps.filter(a => /(Safari|Chrome|Firefox|Edge|Brave|\bArc\b|Opera|Vivaldi|Chromium|Orion|DuckDuckGo|Tor Browser)/i.test(a.name + ' ' + a.bundleId));
      const webLike = shownApps.filter(a => /(\.app\.[a-z]{20,}|web ?clip|Eagle for Safari|safari)/i.test(a.bundleId + ' ' + a.name)).map(a => a.name + ' <' + a.bundleId + '>');
      const te = apps.find(a => a.bundleId === (window.__Q7_APP || 'com.apple.calculator'));
      R.te = te;
      await log('q7-2.1-installed-apps', apps.length > 0 && !shownApps.some(a => REAL_BROWSERS.test(a.name)) && !!te, { total: apps.length, shownAfterBrowserFilter: shownApps.length, browsersLeftInList: browsers.map(a => a.name + ' <' + a.bundleId + '>'), webLike: webLike.slice(0, 12), filteredOut: apps.length - shownApps.length, testApp: te ? { name: te.name, path: te.path, running: te.running } : 'KHÔNG CÓ trong danh sách (thử app: ' + (window.__Q7_APP || 'com.apple.calculator') + ')', sample: apps.slice(0, 12).map(a => a.name) });
      if (!te) throw new Error('app thử không có trong installed_apps');
      const events = []; const evAt = []; const T1 = () => Date.now();
      await inv('activation_subscribe', { onEvent: ch(m => { if (m && m.type === 'appSet') events.push({ at: Date.now(), ...m.event }); }) });
      // fonts bật tay + bật bởi Case B (c2 gồm fA): phải KHÔNG bị tắt theo khi app thoát
      await inv('activation_activate', { faces: [k(fA)], scope: null, proceed: false, onProgress: quiet() });
      await inv('activation_activate_collection', { collectionId: col2, scope: null, proceed: false, onProgress: quiet() });
      const activeSet = async () => new Map((await inv('activation_state', {})).filter(x => x.state === 'active').map(x => [x.hashHex, x]));
      const hasOn = async f => (await activeSet()).has(f.hashHex);
      await inv('case_set_apps', { collectionId: col1, apps: [{ bundleId: te.bundleId, name: te.name }] });
      const saved = await inv('case_apps', { collectionId: col1 });
      await log('q7-2.0-set-apps', saved.length === 1 && saved[0].bundleId === te.bundleId, { saved });
      // 2.2 mở app → font tự bật
      const tOpen = await go('te-open-1');
      const onEv = await wait(() => events.find(e => e.collectionId === col1 && e.kind === 'on'), 90000);
      const bOn = await hasOn(fB), cOn = await hasOn(fC);
      await log('q7-2.2-open-app-turns-on', !!onEv && bOn && cOn, { eventAfterMs: onEv ? onEv.at - tOpen : null, event: onEv || null, faceB: bOn, faceC: cOn });
      // màn "Đang bật" + lý do (bật tay / Case khác / mở cùng app)
      document.querySelector('[data-testid=sidebar] .fa-nav')?.click;
      const navActive = [...document.querySelectorAll('[data-testid=sidebar] .fa-nav, [data-testid=sidebar] button')].find(b => /^(Đang bật|Active)/.test(b.textContent.trim()));
      navActive?.click(); await sleep(1500);
      const screen = textOf(document.querySelector('main, [data-testid=main], .fa-main') || document.body);
      const st = await activeSet(); const rs = h => { const x = st.get(h); return x ? { reasons: x.reasons.map(r => r.kind), names: x.reasonNames } : null; };
      await log('q7-4.1-active-screen-reasons', !!navActive, { A_manual_plus_caseB_plus_app: rs(fA.hashHex), B_app_only: rs(fB.hashHex), C_app_only: rs(fC.hashHex), screenText: screen, rawKeys: rawKeys(screen) });
      await sleep(100);
      // 2.4 thoát app → ~30 s sau font tự tắt
      const tQuit = await go('te-quit-1');
      const offEv = await wait(() => events.find(e => e.collectionId === col1 && e.kind === 'off'), 120000);
      const bOff = !(await hasOn(fB)), cOff = !(await hasOn(fC)), aStill = await hasOn(fA);
      await log('q7-2.4-quit-app-turns-off', !!offEv && bOff && cOff, { secondsAfterQuit: offEv ? ((offEv.at - tQuit) / 1000).toFixed(1) : null, event: offEv || null });
      await log('q7-2.5-manual-and-other-case-stay-on', aStill, { faceA_stillOn: aStill });
      // "Bật lại": bật lại Case sau khi tắt
      const rep = await inv('activation_activate_collection', { collectionId: col1, scope: null, proceed: false, onProgress: quiet() });
      await log('q7-2.4b-turn-on-again', (await hasOn(fB)) && (await hasOn(fC)), { changed: rep.changed });
      await inv('activation_deactivate_all', { onProgress: quiet() }); await sleep(500); // sạch mọi lý do trước khi thử «Tắt lần này»
      // 2.6 "Tắt lần này"
      await go('te-open-2'); const ev2n = events.length;
      await wait(() => events.slice(ev2n).find(e => e.collectionId === col1 && e.kind === 'on'), 90000);
      const onAgain = (await hasOn(fB)) && (await hasOn(fC));
      await inv('case_app_suppress', { collectionId: col1 }); await sleep(2500);
      const afterSup = (await hasOn(fB)) || (await hasOn(fC));
      await log('q7-2.6a-suppress-this-time', onAgain && !afterSup, { onBeforeSuppress: onAgain, anyStillOnAfterSuppress: afterSup });
      await go('te-quit-2'); await wait(() => events.slice(ev2n).find(e => e.collectionId === col1 && e.kind === 'off'), 120000).catch?.(() => null);
      await go('te-open-3'); const ev3n = events.length;
      const back = await wait(() => events.slice(ev3n).find(e => e.collectionId === col1 && e.kind === 'on'), 90000);
      await log('q7-2.6b-reopen-turns-on-again', !!back, { event: back || null, fontsOn: (await hasOn(fB)) && (await hasOn(fC)) });
      await go('te-quit-3'); await wait(() => events.slice(ev3n).find(e => e.collectionId === col1 && e.kind === 'off'), 120000);
      // 2.3 app đã mở sẵn: gắn app SAU khi app đã chạy
      await inv('case_set_apps', { collectionId: col1, apps: [] });
      await go('te-open-early'); const ev4n = events.length;
      await inv('case_set_apps', { collectionId: col1, apps: [{ bundleId: te.bundleId, name: te.name }] });
      const early = await wait(() => events.slice(ev4n).find(e => e.collectionId === col1 && e.kind === 'on'), 40000);
      const earlyOn = (await hasOn(fB)) && (await hasOn(fC));
      await log('q7-2.3-app-already-running', true, { fontsOnWithin40s: earlyOn, event: early || null, note: 'ghi nhận hành vi, không phán đúng/sai' });
      await go('te-quit-4'); await wait(() => events.slice(ev4n).find(e => e.collectionId === col1 && e.kind === 'off'), 120000);
      await inv('case_set_apps', { collectionId: col1, apps: [] });
      await inv('activation_deactivate_all', { onProgress: quiet() });
    } catch (e) { await log('q7-2-appset', false, E(e).slice(0, 500)); }
  }

  // ───────── 3. Font VF trùng tên với bản static ─────────
  if (on('vf')) {
    try {
      const vs = await addSource('vfpair'); const vfRow = (await facesOf(vs.id)).find(f => f.isVariable);
      const stRow = R.smallFaces.find(f => !f.isVariable && f.family === 'Roboto');
      await inv('activation_deactivate_all', { onProgress: quiet() });
      await inv('activation_activate', { faces: [keyOf(stRow)], scope: null, proceed: false, onProgress: quiet() });
      const pf = await inv('activation_preflight', { faces: [keyOf(vfRow)] });
      const kb = await inv('activation_keep_both', { faces: [keyOf(vfRow)], scope: null, onProgress: quiet() });
      const copies = await inv('derived_copies_of', { face: keyOf(vfRow) });
      const act = (await inv('activation_state', {})).filter(x => x.state === 'active');
      const copy = copies[0];
      const staticOn = act.some(x => x.hashHex === stRow.hashHex), vfOn = act.some(x => x.hashHex === vfRow.hashHex), copyOn = copy && act.some(x => x.hashHex === copy.face.hashHex);
      await log('q7-3.1-keep-both', !!copy && staticOn && !vfOn && !!copyOn, { copyName: copy && copy.name, copyPath: copy && copy.path, staticOn, vfOn, copyOn, changed: kb.changed, failed: kb.failed, results: (kb.results || []).slice(0, 3), licenseRestricted: copy && copy.licenseRestricted, preflightNeedsConfirmation: pf.needsConfirmation });
      await go('vf-before-remove', 7000);
      const rm = await inv('remove_derived_copy', { face: copy.face });
      const act2 = (await inv('activation_state', {})).filter(x => x.state === 'active');
      await log('q7-3.3a-remove-copy-to-trash', rm.trashed === true && !act2.some(x => x.hashHex === copy.face.hashHex), { removal: rm, copyStillActive: act2.some(x => x.hashHex === copy.face.hashHex) });
      await go('vf-after-remove', 7000);
      const rs = await inv('restore_derived_copy', { removalId: rm.removalId });
      await log('q7-3.3b-undo-restores', !!rs && !!rs.path, { restore: rs });
      await go('vf-after-restore', 7000);
      await inv('activation_deactivate_all', { onProgress: quiet() });
    } catch (e) { await log('q7-3-vf', false, E(e).slice(0, 500)); }
  }

  // ───────── i18n (en): chuỗi thiếu / dạng khoá thô ─────────
  if (on('i18n')) {
    try {
      window.__faLocale?.('en'); await sleep(800); await openSettings(); await navTo(/(Library|Thư viện)/);
      const t1 = textOf(document.querySelector('[data-testid=organized-panel]'));
      document.querySelector('[data-testid=org-open-import]')?.click(); await wait(() => dlgs().length > 1, 8000); await sleep(800); const t2 = textOf(dlg()); await closeDlg();
      document.querySelector('[data-testid=org-open-relocate]')?.click(); await wait(() => dlgs().length > 1, 8000); await sleep(800); const t3 = textOf(dlg()); await closeDlg();
      document.querySelector('[data-testid=org-open-resort]')?.click(); await wait(() => dlgs().length > 1, 8000); await sleep(800); const t4 = textOf(dlg()); await closeDlg();
      await log('q7-i18n-en-organized', rawKeys(t1 + t2 + t3 + t4).length === 0, { rawKeys: rawKeys(t1 + t2 + t3 + t4), panel: t1, importDlg: t2, relocateDlg: t3, resortDlg: t4 });
      window.__faLocale?.('vi');
    } catch (e) { await log('q7-i18n-en-organized', false, E(e).slice(0, 400)); }
  }

  if (on('sidebar')) {
    const ui = {};
    try {
      const head = [...document.querySelectorAll('[data-testid=sidebar] .fa-sec-head')].find(x => /(Dự án|Projects)/.test(x.textContent));
      const add = head && (head.parentElement.querySelector('.fa-sec-add') || head.nextElementSibling);
      ui.addFound = !!add; add?.click(); await sleep(2500);
      const cases = await inv('list_cases', { includeDeleted: false }); ui.cases = cases.map(c => ({ id: c.id, name: c.name, faces: c.faceCount }));
      const navs = [...document.querySelectorAll('[data-testid=sidebar] .fa-nav')]; ui.navTexts = navs.map(x => x.textContent.trim().slice(0, 30));
      const nav = navs.find(x => cases[0] && x.textContent.includes(cases[0].name)); ui.caseNav = !!nav;
      if (nav) { const r = nav.getBoundingClientRect(); nav.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r.left + 10, clientY: r.top + 5, button: 2 })); }
      await wait(() => document.querySelector('[role=menu]'), 5000);
      const items = [...document.querySelectorAll('[role=menu] [role=menuitem], [role=menu] button')]; ui.menuItems = items.map(x => x.textContent.trim());
      items.find(x => /(Mở cùng app|Open with app)/.test(x.textContent))?.click();
      await wait(() => /(Mở cùng app|Open with app)/.test(textOf(dlg()) || ''), 8000); await sleep(2000);
      const lines = (textOf(dlg()) || '').split('\n').map(x => x.trim()); ui.pickerHead = lines.slice(0, 8); ui.pickerRows = lines.length;
      ui.browserLines = lines.filter(x => /^(Safari|Google Chrome|Firefox|Microsoft Edge|Arc|Brave Browser|Opera|Vivaldi)$/.test(x));
      const inp = dlg()?.querySelector('input'); if (inp) { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(inp, 'Calculator'); inp.dispatchEvent(new Event('input', { bubbles: true })); await sleep(700); }
      const rowEl = [...(dlg() ? dlg().querySelectorAll('label, li, [role=option], [role=checkbox], button') : [])].find(el => /^Calculator\b/.test(el.textContent.trim()));
      ui.rowFound = !!rowEl; (rowEl?.querySelector('input[type=checkbox]') || rowEl)?.click(); await sleep(500);
      const save = btnText(/^(Lưu|Save)/, dlg() || document); ui.saveBtn = save ? save.textContent.trim() : null; save?.click(); await sleep(2000);
      ui.notices = [...document.querySelectorAll('[role=status], [role=alert], .fa-notice, .fa-toast, [data-testid*=notice]')].map(e => e.textContent.trim()).filter(Boolean).slice(0, 4);
      ui.apps = cases[0] ? await inv('case_apps', { collectionId: cases[0].id }).catch(e => 'ERR ' + E(e)) : null;
    } catch (e) { ui.error = E(e); }
    await log('q7-2.1ui-sidebar-open-with', !!ui.rowFound && Array.isArray(ui.apps) && ui.apps.length === 1, ui);
  }
  if (errs.length) await log('q7-console-errors', false, errs.slice(0, 5));
  await log('q7-done', true, null);
})();
