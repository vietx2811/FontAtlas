(async () => {
  const T = window.__TAURI_INTERNALS__;
  const inv = (c, a) => T.invoke(c, a);
  let n = 0; const T0 = Date.now();
  // Lỗi React (ErrorBoundary chỉ console.error): gom lại, đính vào bước FAIL để biết nguyên nhân.
  const consoleErrors = []; const origErr = console.error.bind(console);
  console.error = (...a) => { try { consoleErrors.push(a.map(x => x && x.stack ? String(x.stack) : String(x)).join(' ').slice(0, 800)); } catch (e) {} origErr(...a); };
  window.addEventListener('error', e => consoleErrors.push(String(e.error && e.error.stack || e.message).slice(0, 800)));
  const log = async (step, ok, detail) => { n++; if (!ok && consoleErrors.length) detail = { detail, consoleErrors: consoleErrors.splice(0, 3) }; try { await inv('set_setting', { key: 'ui.qa.' + String(n).padStart(2, '0'), value: JSON.stringify({ step, ok, detail, t: Math.round((Date.now() - T0) / 1000) }) }); } catch (e) {} };
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const wait = async (fn, ms = 20000) => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch (e) {} await sleep(150); } return null; };
  const btn = (text, root = document) => [...root.querySelectorAll('button')].find(b => b.textContent.trim() === text && !b.disabled);
  // Cửa sổ QA có thể không ở trước nên WKWebView không chạy animation/transition: chốt về trạng thái cuối trước khi chụp.
  const settleAnims = () => { try { document.getAnimations().forEach(x => { try { x.finish(); } catch (e) {} }); } catch (e) {} };
  const snap = async name => { settleAnims(); await sleep(80); try { await inv('plugin:event|emit', { event: 'qa-snap', payload: name }); } catch (e) {} await sleep(350); };
  const btnHas = (text, root = document) => [...root.querySelectorAll('button')].find(b => b.textContent.includes(text) && !b.disabled);
  // Ảnh PNG xuất ra tệp qua bảng settings (giới hạn 64 KB/khoá → cắt từng đoạn ui.qapng.<theme>.NN); run.sh ghép lại.
  const savePng = async (th, url) => { const errs = []; if (!url) return ['không có dataUrl']; const b = url.split(',')[1]; for (let i = 0, n = 0; i < b.length; i += 60000, n++) { try { await inv('set_setting', { key: 'ui.qapng.' + th + '.' + String(n).padStart(3, '0'), value: b.slice(i, i + 60000) }); } catch (e) { errs.push('đoạn ' + n + ': ' + String(e && e.message || e).slice(0, 120)); } } return errs; };
  // Thuật ngữ typo: rê chuột vào từng nhãn [data-term], tooltip phải có gloss không rỗng (và hình minh hoạ nếu thuật ngữ có hình).
  const termCheck = async sel => {
    const els = [...document.querySelectorAll(sel)]; const bad = []; let art = 0;
    for (const el of els) {
      el.dispatchEvent(new PointerEvent('pointerover', { bubbles: true, pointerType: 'mouse' }));
      const g = await wait(() => { const x = document.querySelector('[role=tooltip] .fa-term-gloss'); return x && x.textContent.trim() ? x : null; }, 2500);
      if (!g) bad.push(el.dataset.term); else if (document.querySelector('[role=tooltip] svg.fa-term-art')) art++;
      el.dispatchEvent(new PointerEvent('pointerout', { bubbles: true, pointerType: 'mouse' })); await sleep(50);
    }
    return { n: els.length, bad, art };
  };
  try {
    // Chặn hộp thoại chọn thư mục gốc: trả đường dẫn QA (luồng React/IPC còn lại là thật).
    const picks = []; window.__FA_QA_PICKS__ = picks;
    await log('dialog-stub', true, 'window.__FA_QA_PICKS__');
    // Script chạy ngay khi trang load xong: chờ React vẽ onboarding hoặc thư viện (máy bận có thể > 10 s).
    await wait(() => document.querySelector('.fa-onb, .fa-lib'), 90000);
    // ONBOARDING
    let foldersDone = false;
    for (let i = 0; i < 60 && document.querySelector('.fa-onb'); i++) {
      const h1 = document.querySelector('.fa-onb h1')?.textContent ?? '';
      await sleep(600); await snap('onb-' + String(i).padStart(2, '0'));
      const add = btn('+ Thêm thư mục');
      if (add && !foldersDone) {
        const rows = () => document.querySelectorAll('.fa-onb-sources > li:not(.empty)').length;
        const r0 = rows();
        picks.push(['__QA_DIR__/A', '__QA_DIR__/B']); add.click();
        await wait(() => rows() >= r0 + 2);
        await log('onb-add-two-at-once', rows() >= r0 + 2, { before: r0, after: rows(), h1, srcs: (await inv('list_sources', {})).map(s => s.path), ul: document.querySelector('.fa-onb-sources')?.outerHTML.slice(0, 600), alert: document.querySelector('.fa-onb [role=alert], .fa-onb-feedback')?.textContent, picksLeft: picks.length });
        await wait(() => btn('+ Thêm thư mục'));
        picks.push(['__QA_DIR__/C']); btn('+ Thêm thư mục').click();
        await wait(() => rows() >= r0 + 3);
        await log('onb-add-another-later', rows() >= r0 + 3, { after: rows() });
        await wait(() => btn('+ Thêm thư mục'));
        picks.push(['__QA_DIR__/A/']); btn('+ Thêm thư mục').click();
        await sleep(1500);
        const fb = document.querySelector('.fa-onb-feedback, [role=alert]')?.textContent ?? '';
        await log('onb-duplicate-kept', rows() === r0 + 3, { after: rows(), feedback: fb.slice(0, 140) });
        const names = [...document.querySelectorAll('.fa-onb-sources > li:not(.empty)')].map(li => li.textContent.replace(/\s+/g, ' ').slice(0, 80));
        await log('onb-sources-list', names.length === r0 + 3, names);
        await snap('onb-folders-3');
        foldersDone = true;
        const go = await wait(() => btn('Tiếp tục'));
        go?.click(); await sleep(800); continue;
      }
      const order = /Kích hoạt/.test(h1) ? ['Để sau', 'Tiếp tục'] : ['Đã hiểu', 'Không dùng', 'Bỏ qua', 'Tiếp tục', 'Bắt đầu', 'Vào thư viện'];
      const b = await wait(() => order.map(t => btn(t)).find(Boolean), 120000);
      if (!b) { await log('onb-stuck', false, h1); break; }
      b.click(); await sleep(700);
    }
    await log('onb-finished', !document.querySelector('.fa-onb'), '');
    // THƯ VIỆN
    const cells = () => [...document.querySelectorAll('[role=gridcell][data-face-id]')];
    await wait(() => cells().length >= 10, 120000);
    await sleep(1500); await snap('library');
    await log('library-cells', cells().length >= 10, { cells: cells().length, header: document.querySelector('header')?.textContent.replace(/\s+/g, ' ').slice(0, 80) });
    const roboto = cells().find(c => /Roboto/.test(c.textContent) && !/Italic/.test(c.textContent));
    roboto?.click();
    const panel = await wait(() => document.querySelector('[aria-label="Chi tiết font"]'));
    await log('detail-open', !!panel, panel?.querySelector('h2')?.textContent);
    // GIẢI PHẪU
    document.querySelector('[role=tab][aria-label="Giải phẫu"]')?.click();
    const rows = await wait(() => { const r = [...document.querySelectorAll('[data-testid=axis-row]')]; return r.length ? r : null; }, 30000);
    const texts = (rows ?? []).map(r => r.textContent.replace(/\s+/g, ' ').trim());
    const numeric = texts.filter(t => /\d+,\d+\s*(em|°|×)|\d+\s*°/.test(t));
    await snap('anatomy');
    await log('anatomy-numbers', numeric.length >= 5, { rows: texts.length, numeric: numeric.length, sample: texts.slice(0, 8) });
    // Đường kẻ Hxg phải trùng chữ: đo cap của 'H' bằng canvas với đúng font đang vẽ, so với đường cap trong SVG (±1 đơn vị viewBox).
    const hxgCheck = async (step, shot, notFamily = null) => {
      const hxgSvg = await wait(() => { const v = document.querySelector('svg.fa-hxg'); const f = document.querySelector('[data-testid=hxg-text]')?.style.fontFamily; return v && /^(ok|mismatch)$/.test(v.dataset.guides ?? '') && (!notFamily || (f && f !== notFamily)) ? v : null; }, 20000);
      const hxgText = document.querySelector('[data-testid=hxg-text]');
      const capLine = hxgSvg?.querySelector('[data-testid=guide-line][data-line=cap]');
      let inkCap = null;
      if (hxgText) { const c2 = document.createElement('canvas').getContext('2d'); c2.font = '100px ' + hxgText.style.fontFamily; inkCap = c2.measureText('H').actualBoundingBoxAscent; }
      const capY = capLine ? +capLine.getAttribute('y1') : null;
      await snap(shot);
      await log(step, !!hxgSvg && capY !== null && inkCap !== null && Math.abs((112 - inkCap) - capY) <= 1, { capY, expected: inkCap === null ? null : 112 - inkCap, status: hxgSvg?.dataset.guides, family: hxgText?.style.fontFamily, name: document.querySelector('.fa-p-name')?.textContent });
    };
    await hxgCheck('anatomy-hxg-guides-match', 'anatomy-hxg');
    {
      const orig = document.querySelector('[role=gridcell][aria-selected=true]');
      const serifCell = [...document.querySelectorAll('[role=gridcell][data-face-id]')].find(c => /Playfair|Baskerville/.test(c.textContent ?? ''));
      if (serifCell && orig) { const prevFam = document.querySelector('[data-testid=hxg-text]')?.style.fontFamily; serifCell.click(); await hxgCheck('anatomy-hxg-guides-match-serif', 'anatomy-hxg-serif', prevFam); orig.click(); await wait(() => document.querySelector('[data-testid=hxg-text]')?.style.fontFamily === prevFam, 8000); }
      else await log('anatomy-hxg-guides-match-serif', false, { error: 'không tìm thấy ô serif hoặc ô đang chọn (SKIPPED, không tính là pass)', hasSerif: !!serifCell, hasOrig: !!orig });
    }
    const termA = await termCheck('[data-testid=axis-row] [data-term]');
    await log('anatomy-term-tooltips', termA.n >= 5 && termA.bad.length === 0 && termA.art > 0, termA);
    const axisNames = [...document.querySelectorAll('[data-testid=axis-row] .fa-axis-name')].map(e => ({ text: e.textContent.trim(), term: e.dataset.term }));
    await log('anatomy-labels-english', axisNames.length >= 10 && axisNames.every(n => n.term && !/[À-ỹ]/.test(n.text)), { total: axisNames.length, bad: axisNames.filter(n => !n.term || /[À-ỹ]/.test(n.text)), sample: axisNames.slice(0, 8).map(n => n.text) });
    // TƯƠNG TỰ
    document.querySelector('[role=tab][aria-label="Tương tự"]')?.click();
    const acts = await wait(() => { const a = [...document.querySelectorAll('.fa-sim-actions')]; return a.length ? a : null; }, 60000);
    const items = (acts ?? []).map(a => a.closest('li, article, div')?.parentElement?.textContent.replace(/\s+/g, ' ').slice(0, 70));
    const fams = (acts ?? []).map(a => (a.closest('li, article') ?? a.parentElement)?.querySelector('h3, h4, strong, b')?.textContent ?? '');
    await sleep(800); await snap('similar');
    await log('similar-families', !!acts && !fams.some(f => /^Roboto$/.test(f.trim())), { count: acts?.length ?? 0, fams, hasOpen: !!btn('Mở font'), hasFamily: !!btnHas('Xem family') });
    // XEM FAMILY
    const xf = btnHas('Xem family'); const target = fams[0];
    xf?.click();
    const chip = await wait(() => document.querySelector('.fa-family-chip'), 20000);
    await sleep(1200);
    await snap('family-chip');
    await log('view-family', !!chip, { chip: chip?.textContent, title: chip?.getAttribute('aria-label'), target, cells: cells().length, cellNames: cells().slice(0, 6).map(c => c.textContent.replace(/\s+/g, ' ').slice(-40)) });
    // LỌC
    const fbtn = document.querySelector('button[aria-controls="font-filters"]');
    fbtn?.click();
    const fp = await wait(() => document.getElementById('font-filters'));
    const inputs = fp ? [...fp.querySelectorAll('input[type=text], input[type=number], input[inputmode]')].slice(0, 4).map(i => i.value) : [];
    const ranges = fp ? [...fp.querySelectorAll('p, small, span')].map(e => e.textContent).filter(t => /Min .* max/.test(t)).slice(0, 3) : [];
    const before = cells().length;
    const it = fp && [...fp.querySelectorAll('label')].find(l => (l.textContent.includes('Italic') || l.textContent.includes('In nghiêng')))?.querySelector('input');
    it?.click();
    await sleep(1500);
    const chip2 = document.querySelector('.fa-family-chip');
    await snap('filter-italic');
    await log('filter-italic', !!fp && !!it, { inputs, ranges, before, after: cells().length, chip: chip2?.getAttribute('aria-label'), trigger: fbtn?.textContent });
    it?.click(); await sleep(1000);
    document.querySelector('[aria-label="Đóng bộ lọc"]')?.click();
    await log('filter-cleared', true, { cells: cells().length, chip: document.querySelector('.fa-family-chip')?.getAttribute('aria-label') });
    // BỘ LỌC trên toàn thư viện: khoảng slider lấy p1–p99 của chính thư viện, không phải 0–1
    document.querySelector('.fa-family-chip')?.click(); await sleep(1000);
    document.querySelector('button[aria-controls="font-filters"]')?.click();
    const fpAll = await wait(() => document.getElementById('font-filters'));
    await sleep(800); await snap('filter-full-library');
    const rangesAll = fpAll ? [...fpAll.querySelectorAll('p, small, span')].map(e => e.textContent).filter(t => /Min .* max/.test(t)).slice(0, 6) : [];
    await log('filter-full-library', !!fpAll && rangesAll.length >= 3 && !document.querySelector('.fa-family-chip'), { cells: cells().length, rangesAll });
    document.querySelector('[aria-label="Đóng bộ lọc"]')?.click(); await sleep(300);
    // CÀI ĐẶT → SIDEBAR
    document.querySelector('button[aria-label="Cài đặt"]')?.click();
    const st = await wait(() => document.querySelector('[data-testid=settings]'));
    await snap('settings');
    const nav = [...document.querySelectorAll('[aria-label="Thanh bên"] button, nav button, aside button')].find(b => b.textContent.includes('Tất cả'));
    nav?.click();
    const left = await wait(() => !document.querySelector('[data-testid=settings]') && cells().length > 0, 10000);
    await sleep(800); await snap('after-settings');
    await log('settings-sidebar-leaves', !!st && !!left, { settingsOpened: !!st, navFound: !!nav, cells: cells().length });

    // ===== TIÊU ĐỀ BẰNG FONT CỦA THẺ · TOOLTIP · ĐA CHỌN · MENU NGỮ CẢNH (sáng) =====
    document.documentElement.dataset.theme = 'light';
    const key = c => c.querySelector('[data-testid=face-name]');
    const uiFont = getComputedStyle(document.body).fontFamily;
    const hover = el => el.dispatchEvent(new PointerEvent('pointerover', { bubbles: true, pointerType: 'mouse' }));
    const unhover = el => el.dispatchEvent(new PointerEvent('pointerout', { bubbles: true, pointerType: 'mouse' }));
    const tip = () => document.querySelector('[role=tooltip]');
    const ownNames = () => cells().map(key).filter(n => n && n.dataset.ownFont);
    await wait(() => cells().length >= 10, 30000);
    await wait(() => ownNames().length >= 3, 30000);
    await sleep(100); await snap('cards-own-font');
    const ownFams = ownNames().slice(0, 6).map(n => ({ text: n.textContent, family: getComputedStyle(n).fontFamily.slice(0, 40), differs: getComputedStyle(n).fontFamily !== uiFont, hasFallback: /Inter|var\(--font-ui\)|-apple-system/.test(getComputedStyle(n).fontFamily) || getComputedStyle(n).fontFamily.split(',').length > 1 }));
    await log('card-names-own-font', ownFams.length >= 3 && ownFams.every(f => f.differs), { loaded: ownNames().length, of: cells().length, ownFams });
    // Tooltip: rê chuột vào chấm tiếng Việt của thẻ đầu, rồi vào nút Cài đặt
    const vi0 = cells()[0].querySelector('[data-testid=vi-mark]');
    hover(vi0);
    const t1 = await wait(tip, 3000);
    const t1Text = t1?.textContent; await sleep(100); await snap('tooltip-badge');
    unhover(vi0); await sleep(200);
    const gear = document.querySelector('button[aria-label="Cài đặt"]');
    hover(gear);
    const t2 = await wait(tip, 3000);
    const t2Text = t2?.textContent; await sleep(100); await snap('tooltip-settings');
    unhover(gear); await sleep(200);
    await log('tooltip-hover', !!t1 && !!t2 && /Cài đặt/.test(t2Text) && /⌘,/.test(t2Text) && !tip(), { badge: t1Text, gear: t2Text, leftover: !!tip(), nativeTitles: document.querySelectorAll('[title]').length });
    // Đa chọn: click + ⌘-click. Đo lưới so với panel chi tiết trước/sau khi panel mở và thanh chọn xuất hiện.
    const r1 = n => Math.round(n * 10) / 10;
    const rect = el => { if (!el) return null; const r = el.getBoundingClientRect(); return { l: r1(r.left), r: r1(r.right), t: r1(r.top), b: r1(r.bottom), w: r1(r.width) }; };
    const measure = label => {
      const sc = document.querySelector('[data-testid=library-grid-scroller]'); const grid = document.querySelector('[role=grid]');
      const lib = document.querySelector('.fa-lib'); const panel = document.querySelector('aside.fa-panel');
      const size = Number(document.querySelector('.fa-composer input[type=range]')?.value ?? 34);
      const cols = Number(grid?.getAttribute('aria-colcount'));
      const expected = grid ? Math.max(1, Math.floor((grid.getBoundingClientRect().width - 28) / (140 + size * 2.6 + 8))) : null;
      const sr = sc?.getBoundingClientRect(); const pr = panel?.getBoundingClientRect();
      return { label, scroller: rect(sc), grid: rect(grid), lib: rect(lib), panel: rect(panel), cols, expected, fits: !!sr && (!pr || sr.right <= pr.left + 1), colsOk: cols === expected, hidden: document.hidden, focus: document.hasFocus() };
    };
    // rAF có thể không chạy khi cửa sổ QA ở nền → hẹn giờ thật để không treo; số tick = 0 chứng minh WebView đang không dựng khung.
    const rafTicks = ms => new Promise(res => { let n = 0; const t0 = performance.now(); const f = () => { n++; if (performance.now() - t0 < ms) requestAnimationFrame(f); }; requestAnimationFrame(f); setTimeout(() => res(n), ms + 100); });
    const cs = cells();
    const m0 = measure('before-panel');
    cs[0].click(); await sleep(300);
    const m1 = measure('panel-open+300ms');
    await sleep(500);
    const m1b = measure('panel-open+800ms');
    cs[1].dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, metaKey: true }));
    const bar = await wait(() => document.querySelector('[data-testid=selection-count]'), 5000);
    await sleep(100); await snap('selection-bar');
    await log('cmd-click-two', /^2 đã chọn$/.test(bar?.textContent ?? ''), { bar: bar?.textContent, selected: cells().filter(c => c.getAttribute('aria-selected') === 'true').length });
    const m2 = measure('bar-shown');
    await sleep(500);
    const m3 = measure('bar-shown+500ms');
    const ticks = await rafTicks(500);
    const m4 = measure('after-raf');
    await log('grid-fits-panel', [m1, m1b, m2, m3, m4].every(m => m.fits && m.colsOk), { rafTicksIn500ms: ticks, m0, m1, m1b, m2, m3, m4 });
    const barBox = bar?.closest('.fa-selbar')?.getBoundingClientRect(); const gridBox = document.querySelector('[role=grid]')?.getBoundingClientRect(); const hdr = document.querySelector('header.fa-topbar');
    await log('selection-bar-in-row', !!barBox && !!gridBox && barBox.bottom <= gridBox.top + 1 && !!hdr?.contains(bar) && !!bar.closest('.fa-family-row'), { bar: barBox && [Math.round(barBox.top), Math.round(barBox.bottom)], gridTop: gridBox && Math.round(gridBox.top), header: hdr?.className });

    // Top Bar: 2 font đang chọn → Compare · Pair these (không có Find similar)
    const pcGroup = document.querySelector('header.fa-topbar [role=group][aria-label="Pair / Compare"]');
    const actNames = [...(pcGroup?.querySelectorAll('button') ?? [])].map(b => b.getAttribute('aria-label'));
    await snap('topbar-actions-2');
    await log('topbar-actions-two', !!pcGroup && actNames.includes('Compare') && actNames.includes('Pair these') && !actNames.includes('Find similar') && /2 chọn/.test(pcGroup.textContent), { actNames, text: pcGroup?.textContent });

    // Compare (⇧C / nút) với 2 font: màn Compare thật, số đo thật, Esc quay lại giữ vùng chọn
    const compareBtn = [...(pcGroup?.querySelectorAll('button') ?? [])].find(x => x.getAttribute('aria-label') === 'Compare');
    compareBtn?.click();
    const tbl = await wait(() => document.querySelector('.cmp-tbl'), 30000);
    await wait(() => document.querySelectorAll('.cmp-tbl .cmp-metric, .cmp-tbl .cmp-na').length >= 8, 30000);
    await sleep(600); await snap('compare-open');
    const colHeads = [...document.querySelectorAll('.cmp-tbl .ch')];
    const metricCells = document.querySelectorAll('.cmp-tbl .cmp-metric').length;
    const ovlCount = 2; // Overlay không còn là hàng; kiểm ở các bước overlay bên dưới
    const crashed = /Lỗi giao diện/.test(document.body.textContent ?? '');
    const sizeBefore = [...document.querySelectorAll('.cmp-line')].map(x => x.style.fontSize);
    btn('Match x-height')?.click(); await sleep(300);
    const sizeAfter = [...document.querySelectorAll('.cmp-line')].map(x => x.style.fontSize);
    const xhRow = [...document.querySelectorAll('.cmp-tbl .row')].find(r => /x-height/.test(r.querySelector('.rh')?.textContent ?? ''));
    const xhVals = [...(xhRow?.querySelectorAll('.cmp-val b') ?? [])].map(b => b.textContent);
    const colResized = sizeAfter[1] !== sizeBefore[1] || xhVals[0] === xhVals[1]; // cột 2 đổi cỡ trừ khi x-height hai font bằng nhau
    await snap('compare-match-xheight');
    await log('compare-open', !!tbl && colHeads.length === 2 && colHeads[0].classList.contains('base') && metricCells >= 4 && ovlCount === 2 && !crashed && sizeBefore[0] === sizeAfter[0] && colResized, { colResized, xhVals, cols: colHeads.length, metricCells, ovlCount, crashed, sizeBefore, sizeAfter, text: tbl?.textContent.slice(0, 200) });
    // ── Compare v2 (H1–H8) ──
    const ariaBtn = (pre, root = document) => [...root.querySelectorAll('button')].find(b => (b.getAttribute('aria-label') ?? '').startsWith(pre));
    const heads = () => [...document.querySelectorAll('.cmp-tbl .ch')];
    const headName = h => h?.querySelector('.t span')?.textContent;
    // H1: cột property neo ngang (computed style thật trong WKWebView)
    const rhCs = getComputedStyle(document.querySelector('.cmp-tbl .rh[role=rowheader]')); const tblCs = getComputedStyle(document.querySelector('.cmp-tbl'));
    await log('compare-sticky-property-col', rhCs.position === 'sticky' && rhCs.left === '0px' && tblCs.overflowX !== 'hidden' && parseFloat(rhCs.zIndex) >= 2, { pos: rhCs.position, left: rhCs.left, tblOverflow: tblCs.overflowX, z: rhCs.zIndex });
    // H2: pin
    const namesBefore = heads().map(headName);
    ariaBtn('Pin cột', heads()[1])?.click(); await sleep(300);
    const afterPin = heads().map(headName);
    await snap('compare-v2-pinned');
    await log('compare-pin', afterPin[0] === namesBefore[1] && heads()[0].dataset.pinned === 'true' && heads()[1].classList.contains('base') && /mốc/.test(heads()[1].textContent), { namesBefore, afterPin });
    ariaBtn('Bỏ pin', heads()[0])?.click(); await sleep(300);
    // H3: + thêm font, × bỏ
    const addBtn = [...document.querySelectorAll('.cmp-tbl button')].find(b => b.getAttribute('aria-label') === 'Thêm font');
    addBtn?.click();
    const dlg = await wait(() => document.querySelector('.cmp-picker'), 8000);
    const opt = await wait(() => dlg?.querySelector('button[role=option]'), 15000);
    const optName = opt?.textContent;
    await snap('compare-v2-picker');
    opt?.click(); await wait(() => heads().length === 3, 15000);
    const added = heads().length;
    await wait(() => document.querySelectorAll('.cmp-tbl .cmp-metric, .cmp-tbl .cmp-na').length >= 12, 20000);
    ariaBtn('Bỏ font này khỏi so sánh', heads()[2])?.click(); await wait(() => heads().length === 2, 8000);
    await log('compare-add-remove', !!dlg && !!optName && added === 3 && heads().length === 2 && !document.querySelector('.cmp-picker'), { optName, added, after: heads().length });
    // H6: sửa specimen tại chỗ, áp dụng mọi cột; ký tự thiếu xám + NoteMark
    const prevSample = document.querySelector('.cmp-edit')?.value;
    const setVal = (el, v) => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(el, v); el.dispatchEvent(new Event('input', { bubbles: true })); };
    setVal(document.querySelector('.cmp-edit'), 'Quạ đen 龍');
    await wait(() => [...document.querySelectorAll('.cmp-spec-view')].every(v => v.textContent === 'Quạ đen 龍'), 8000);
    const specNotes = () => [...document.querySelectorAll('.cmp-spec .fa-note')].filter(n => /^Thiếu:/.test(n.getAttribute('aria-label') ?? ''));
    await wait(() => specNotes().length >= document.querySelectorAll('.cmp-edit').length && document.querySelectorAll('.cmp-spec-view').length === [...document.querySelectorAll('.cmp-spec-view')].filter(v => v.querySelector('.miss')).length, 20000); // mọi cột đều thiếu 龍 (font Latin)
    const views = [...document.querySelectorAll('.cmp-spec-view')].map(v => v.textContent);
    const missNote = [...document.querySelectorAll('.cmp-spec .fa-note')].map(n => n.getAttribute('aria-label')).find(x => /^Thiếu:/.test(x ?? ''));
    const missCol = getComputedStyle(document.querySelector('.cmp-spec .miss')).color; const inkCol = getComputedStyle(document.querySelector('.cmp-spec-view')).color;
    await snap('compare-v2-edit');
    await log('compare-edit-specimen', document.querySelectorAll('.cmp-edit').length === 2 && views.every(v => v === 'Quạ đen 龍') && document.querySelector('.cmp-edit').value === 'Quạ đen 龍' && !!missNote && missCol !== inkCol && specNotes().length === 2 && [...document.querySelectorAll('.cmp-spec-view')].every(v => v.querySelector('.miss')?.textContent === '龍' || [...v.querySelectorAll('.miss')].some(m => m.textContent === '龍')), { views, missNote, missCol, inkCol, miss: document.querySelectorAll('.cmp-spec .miss').length });
    // Chẩn đoán phát hiện glyph thiếu trong WKWebView thật (ghi số liệu để chỉnh phép thử)
    try {
      const famName = (getComputedStyle(document.querySelector('.cmp-spec-view')).fontFamily.match(/"([^"]+)"/) ?? [])[1];
      const cv = document.createElement('canvas'); cv.width = 80; cv.height = 64; const g = cv.getContext('2d');
      const stacks = { famSerif: `40px "${famName}", serif`, famSans: `40px "${famName}", sans-serif`, famMono: `40px "${famName}", monospace`, famOnly: `40px "${famName}"`, noneSerif: '40px "__none__", serif', noneSans: '40px "__none__", sans-serif', noneOnly: '40px "__none__"' };
      const diag = {};
      for (const ch of ['龍', 'ạ', 'Q']) {
        diag[ch] = {};
        for (const [k, f] of Object.entries(stacks)) { g.clearRect(0, 0, 80, 64); g.font = f; g.fillStyle = '#000'; g.fillText(ch, 4, 48); const d = g.getImageData(0, 0, 80, 64).data; let sum = 0, h = 0; for (let i = 3; i < d.length; i += 4) { sum += d[i]; h = (h * 31 + d[i]) | 0; } g.font = f.replace('40px', '64px'); diag[ch][k] = `${sum}/${h}/${g.measureText(ch).width.toFixed(1)}`; }
      }
      await log('compare-glyph-diag', false, { famName, diag, fontsCheck: document.fonts.check(`40px "${famName}"`, '龍') });
    } catch (e) { await log('compare-glyph-diag', false, { err: String(e) }); }
    // PNG giữ ký tự thiếu: xám + chữ ghi chú (ảnh không có hover)
    const pngMiss = await (window.__faQaComparePng ? window.__faQaComparePng() : null);
    await log('compare-png-missing-note', !!pngMiss && pngMiss.texts.some(x => /^Thiếu: /.test(x)), { notes: pngMiss?.texts.filter(x => /Thiếu/.test(x)) });
    // Esc của bộ gõ (isComposing/229) trong ô specimen không đóng màn Compare
    const ed = document.querySelector('.cmp-edit'); ed.focus();
    ed.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', keyCode: 229, isComposing: true, bubbles: true, cancelable: true })); await sleep(300);
    await log('compare-esc-ime-stays-open', !!document.querySelector('.cmp-tbl') && document.activeElement === ed, { open: !!document.querySelector('.cmp-tbl') });
    ed.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true })); await sleep(200); // Esc thường trong ô: chỉ blur, màn vẫn mở
    await log('compare-esc-in-field-blurs', document.activeElement !== ed && !!document.querySelector('.cmp-tbl'), {});
    setVal(document.querySelector('.cmp-edit'), prevSample ?? 'Phở bò Hà Nội'); await sleep(300);
    // H7: hàng Vietnamese có TermTip "Stacked diacritics" + chữ "dấu chồng"; ký tự dấu chồng được đánh dấu
    const viHead = document.querySelector('[data-row="specimen.vi"]');
    await log('compare-vi-row', /Vietnamese/.test(viHead?.querySelector('b')?.textContent ?? '') && viHead?.querySelector('[data-term="vi_marks"]')?.textContent === 'Stacked diacritics' && /dấu chồng/.test(viHead?.textContent ?? '') && document.querySelectorAll('.cmp-vi [data-stacked="true"]').length >= 5, { head: viHead?.textContent, stacked: document.querySelectorAll('.cmp-vi [data-stacked="true"]').length });
    // H8a: Overlay là vùng chung, bật bằng nút; mắt trên tiêu đề cột chọn font; mốc đặc
    const ovBtn = btn('Overlay');
    ovBtn?.click(); await wait(() => document.querySelector('.cmp-ovl text'), 8000); await sleep(500);
    const styles1 = [...document.querySelectorAll('.cmp-ovl text')].map(x => x.getAttribute('data-style'));
    await snap('compare-v2-overlay');
    const eye1 = ariaBtn('Overlay ·', heads()[1]); eye1?.click(); await sleep(300);
    const styles2 = [...document.querySelectorAll('.cmp-ovl text')].map(x => x.getAttribute('data-style'));
    eye1?.click(); await sleep(300);
    const pngOv = await (window.__faQaComparePng ? window.__faQaComparePng() : null);
    await log('compare-overlay-toggle', ovBtn?.getAttribute('aria-pressed') === 'true' && styles1.join() === 'solid,line' && styles2.join() === 'solid' && !document.querySelector('[data-row="specimen.overlay"]') && !!pngOv && pngOv.overlayLayers === 2 && pngOv.rows.includes('overlay'), { styles1, styles2, pngLayers: pngOv?.overlayLayers });
    // H8b: thanh đo theo min–max của các font đang xem: hai marker không dính nhau khi giá trị khác nhau
    const bars = [...document.querySelectorAll('.cmp-tbl .row')].filter(r => r.querySelectorAll('.cmp-pbar').length === 2).map(r => [...r.querySelectorAll('.cmp-pbar i')].map(i => parseFloat(i.style.width)));
    await log('compare-pbar-scale', bars.length > 0 && bars.some(b => Math.abs(b[0] - b[1]) > 5) && !document.querySelector('.cmp-pbar em'), { rows: bars.length, spreads: bars.slice(0, 6) });
    // Ảnh dark cho bảng v2 (overlay + pin + thanh tô từ 0)
    const themeV2 = document.documentElement.dataset.theme; document.documentElement.dataset.theme = "dark"; await sleep(400);
    await snap('compare-v2-dark');
    if (themeV2) document.documentElement.dataset.theme = themeV2; else document.documentElement.removeAttribute("data-theme"); await sleep(200);
    const termC = await termCheck('.cmp-tbl .rh [data-term]');
    await log('compare-term-tooltips', termC.n >= 10 && termC.bad.length === 0 && termC.art >= 8, termC);
    // PNG: vẽ như nút Xuất nhưng không qua hộp thoại (móc QA); phải có overlay, đơn vị, nhãn không bị cắt.
    const rgbSum = el => { const m = (el ? getComputedStyle(el).backgroundColor : '').match(/\d+/g); return m ? m.slice(0, 3).reduce((a, b) => a + +b, 0) : -1; };
    const lightSpec = rgbSum(document.querySelector('.cmp-page'));
    const pngL = await (window.__faQaComparePng ? window.__faQaComparePng() : null);
    // Lưu ảnh PNG thật (mặt chữ thật) vào bảng settings (khoá ui.qapng.*, không trùng ui.qa.%); run.sh ghi ra tệp.
    const saveErrL = await savePng('light', pngL?.dataUrl);
    await log('compare-png-content', !!pngL && pngL.rows.includes('overlay') && pngL.rows.includes('vi') && pngL.texts.some(x => /\d,\d+ (em|°|tỷ lệ|×)/.test(x)) && !pngL.texts.some(x => x.includes('…')) && pngL.dark === false, { rows: pngL?.rows, units: pngL?.texts.filter(x => /\d,\d+ /.test(x)).slice(0, 4), ellipsis: pngL?.texts.filter(x => x.includes('…')), warn: pngL?.texts.filter(x => /Thiếu|Dấu/.test(x)) });
    // DARK: vùng specimen nền tối, đổi theme lúc đang mở là đổi ngay; PNG theo theme.
    const prevTheme = document.documentElement.dataset.theme;
    document.documentElement.dataset.theme = 'dark'; await sleep(200);
    const darkSpec = rgbSum(document.querySelector('.cmp-page'));
    const darkInk = getComputedStyle(document.querySelector('.cmp-page')).color;
    const pngD = await (window.__faQaComparePng ? window.__faQaComparePng() : null);
    const saveErrD = await savePng('dark', pngD?.dataUrl);
    await log('compare-png-saved', saveErrL.length === 0 && saveErrD.length === 0, { light: saveErrL, dark: saveErrD });
    await snap('compare-dark');
    await log('compare-dark-specimen', lightSpec > 700 && darkSpec >= 0 && darkSpec < 60 && /^rgb\(25[0-9]/.test(darkInk) && !!pngD && pngD.dark === true && pngD.specimenBg.toLowerCase() === '#0a0908', { lightSpec, darkSpec, darkInk, pngDark: pngD?.dark, bg: pngD?.specimenBg });
    // Sidebar dark: mục đang chọn phải đọc được (tương phản chữ/nền ≥ 4.5:1) và nền là màu đặc, không phải khối sáng.
    const lumOf = rgb => { const [r, g, bl] = rgb.map(v => { v /= 255; return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4); }); return 0.2126 * r + 0.7152 * g + 0.0722 * bl; };
    const parseC = c => { const m = (c ?? '').match(/[\d.]+/g); return m ? m.map(Number) : null; };
    const navSel = document.querySelector('.fa-side .fa-nav[aria-current="true"]');
    const navBg = parseC(navSel && getComputedStyle(navSel).backgroundColor); const navFg = parseC(navSel && getComputedStyle(navSel).color);
    let navRatio = null;
    if (navBg && navFg) { const [l1, l2] = [lumOf(navBg), lumOf(navFg)].sort((p, q) => q - p); navRatio = (l1 + 0.05) / (l2 + 0.05); }
    const themeBtnF = document.querySelector('.fa-side [aria-label="Giao diện sáng/tối"]');
    const tb = getComputedStyle(themeBtnF ?? document.body); const tbBg = parseC(tb.backgroundColor); const tbFg = parseC(tb.color);
    const tbRatio = tbBg && tbFg ? (() => { const [l1, l2] = [lumOf(tbBg), lumOf(tbFg)].sort((p, q) => q - p); return (l1 + 0.05) / (l2 + 0.05); })() : null;
    await log('sidebar-selected-contrast-dark', !!navSel && navBg && navBg.length >= 3 && (navBg[3] === undefined || navBg[3] === 1) && navRatio >= 4.5 && lumOf(navBg) < 0.12 && tbRatio !== null && tbRatio >= 3, { navBg, navFg, navRatio, tbBg, tbFg, tbRatio });
    if (prevTheme) document.documentElement.dataset.theme = prevTheme; else document.documentElement.removeAttribute('data-theme');
    (document.activeElement ?? document.body).dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    await wait(() => !document.querySelector('.cmp-tbl') && document.querySelector('[data-testid=selection-count]'), 8000);
    await log('compare-close-keeps-selection', !document.querySelector('.cmp-tbl') && /^2 đã chọn$/.test(document.querySelector('[data-testid=selection-count]')?.textContent ?? ''), { count: document.querySelector('[data-testid=selection-count]')?.textContent });

    // Pair these (2 font): màn Combo thật từ command pair_these; Esc về thư viện giữ vùng chọn
    const pairBtn = [...document.querySelectorAll('header.fa-topbar [role=group][aria-label="Pair / Compare"] button')].find(x => x.getAttribute('aria-label') === 'Pair these');
    pairBtn?.click();
    const prRoot = await wait(() => document.querySelector('.pr'), 20000);
    await wait(() => document.querySelector('[data-testid=pair-combo], [data-testid=zero-state], .pr-state[role=alert]'), 30000);
    await sleep(500); await snap('pair-these-open');
    const comboN = document.querySelectorAll('[data-testid=pair-combo]').length;
    const prAlert = document.querySelector('.pr-state[role=alert]')?.textContent ?? null;
    const prLabels = document.querySelectorAll('[data-testid=pair-label]').length;
    const prCrashed = /Lỗi giao diện/.test(document.body.textContent ?? '');
    await log('pair-these-open', !!prRoot && !prAlert && !prCrashed && (comboN === 0 ? !!document.querySelector('[data-testid=zero-state]') : comboN <= 3 && prLabels >= comboN), { comboN, prLabels, prAlert, prCrashed, first: document.querySelector('[data-testid=pair-summary]')?.textContent });
    (document.activeElement ?? document.body).dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    await wait(() => !document.querySelector('.pr') && document.querySelector('[data-testid=selection-count]'), 8000);
    await log('pair-these-close-keeps-selection', !document.querySelector('.pr') && /^2 đã chọn$/.test(document.querySelector('[data-testid=selection-count]')?.textContent ?? ''), { count: document.querySelector('[data-testid=selection-count]')?.textContent });

    // Menu ngữ cảnh trong vùng chọn → giữ vùng chọn, menu nhiều font
    const r0 = cs[0].getBoundingClientRect();
    cs[0].dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r0.left + 60, clientY: r0.top + 60, button: 2 }));
    const menu = await wait(() => document.querySelector('[role=menu]'), 3000);
    await sleep(100); await snap('context-menu');
    const labels = [...document.querySelectorAll('[role=menuitem]')].map(i => i.textContent);
    await log('context-menu-open', !!menu && labels.some(l => /^Thêm vào dự án/.test(l)) && !!bar && document.querySelector('[data-testid=selection-count]')?.textContent === '2 đã chọn', { labels, header: menu?.textContent.slice(0, 30) });
    const addProj = [...document.querySelectorAll('[role=menuitem]')].find(i => /^Thêm vào dự án/.test(i.textContent));
    addProj?.click();
    const newProj = await wait(() => [...document.querySelectorAll('[role=menuitem]')].find(i => i.textContent === 'Dự án mới…'), 3000);
    await sleep(100); await snap('context-submenu');
    newProj?.click();
    const nameInput = await wait(() => document.querySelector('input[aria-label="Tên dự án"]'), 3000);
    await sleep(100); await snap('new-project-prompt');
    if (nameInput) {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(nameInput, 'QA hai font');
      nameInput.dispatchEvent(new Event('input', { bubbles: true }));
      nameInput.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
    }
    let proj = null;
    for (let i = 0; i < 30 && !proj?.itemCount; i++) { await sleep(300); proj = (await inv('list_collections', {})).find(c => c.name === 'QA hai font'); }
    await sleep(100); await snap('project-created');
    await log('new-project-has-two-fonts', !!proj && proj.itemCount === 2, { proj, toast: document.querySelector('.fa-toast')?.textContent, sidebar: [...document.querySelectorAll('[aria-label="Thanh bên"] button')].some(b => /QA hai font/.test(b.textContent)) });
    // Esc bỏ chọn: chờ menu/hộp nhập/tooltip đã đóng (điều kiện thật, không đoán thời gian), rồi gửi Esc tới phần tử đang focus như bàn phím thật.
    const overlays = () => document.querySelector('[role=menu], [role=dialog], [role=alertdialog], [role=tooltip]');
    const idle = await wait(() => !overlays() && document.querySelector('[data-testid=selection-count]'), 10000);
    (document.activeElement ?? document.body).dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    const cleared = await wait(() => !document.querySelector('[data-testid=selection-count]') && cells().every(c => c.getAttribute('aria-selected') !== 'true'), 8000);
    await log('escape-clears', !!idle && !!cleared, { idleBeforeEsc: !!idle, left: cells().filter(c => c.getAttribute('aria-selected') === 'true').length, overlay: overlays()?.outerHTML.slice(0, 160) ?? null, active: document.activeElement && (document.activeElement.tagName + '.' + String(document.activeElement.className).slice(0, 40)), bar: document.querySelector('[data-testid=selection-count]')?.textContent ?? null });
    // Chuột phải thẻ chưa chọn (1 font) + tiêu đề panel bằng font của thẻ + dark
    const c3 = cells()[3]; const r3 = c3.getBoundingClientRect();
    c3.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r3.left + 50, clientY: r3.top + 50, button: 2 }));
    await wait(() => document.querySelector('[role=menu]'), 3000);
    await sleep(400);
    const single = [...document.querySelectorAll('[role=menuitem]')].map(i => i.textContent);
    const h2 = document.querySelector('[aria-label="Chi tiết font"] h2');
    await wait(() => h2 && h2.dataset.ownFont, 8000);
    await snap('context-menu-single');
    await log('context-menu-single', single.includes('Mở chi tiết') && single.includes('Tìm tương tự') && cells().filter(c => c.getAttribute('aria-selected') === 'true').length === 1, { single, panelTitleOwnFont: h2?.dataset.ownFont ?? null, panelTitleFamily: h2 && getComputedStyle(h2).fontFamily.slice(0, 40) });
    (document.activeElement ?? document.body).dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    await wait(() => !document.querySelector('[role=menu]'), 5000);
    document.documentElement.dataset.theme = 'dark';
    await sleep(150);
    c3.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r3.left + 50, clientY: r3.top + 50, button: 2 }));
    await wait(() => document.querySelector('[role=menu]'), 3000);
    await sleep(100); await snap('context-menu-dark');
    document.body.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true }));
    const gear2 = document.querySelector('button[aria-label="Cài đặt"]'); hover(gear2); await wait(tip, 3000); await sleep(100); await snap('tooltip-dark'); unhover(gear2);
    document.documentElement.dataset.theme = 'light';
    await log('dark-snapshots', true, '');
    // Nút sáng/tối ở sidebar phải đổi cả appearance cửa sổ native (lớp kính vibrancy của sidebar)
    const themeBtn = document.querySelector('button[aria-label="Giao diện sáng/tối"]');
    const nativeTheme = () => inv('plugin:window|theme', { label: 'main' }).catch(e => 'err:' + e);
    const themeBefore = await nativeTheme();
    themeBtn?.click(); await sleep(400);
    const css1 = document.documentElement.dataset.theme, native1 = await nativeTheme();
    themeBtn?.click(); await sleep(400);
    const css2 = document.documentElement.dataset.theme, native2 = await nativeTheme();
    await log('native-theme-follows', css1 === native1 && css2 === native2, { themeBefore, css1, native1, css2, native2 });
    // Phân loại: chip ở đầu panel + lọc theo class bằng facet thật
    const cchip = await wait(() => document.querySelector('[data-testid=class-chip]'), 10000);
    await sleep(150); await snap('class-chip');
    const chipText = (cchip?.textContent ?? '').trim();
    const countNow = () => Number((document.querySelector('[data-testid=view-count]')?.textContent ?? '').replace(/\D/g, ''));
    const total0 = countNow();
    const filterBtn = [...document.querySelectorAll('button')].find(x => /^Lọc/.test(x.textContent.trim()));
    filterBtn?.click();
    const grp = await wait(() => document.querySelector('[aria-label="Phân loại"]'), 10000);
    const boxes = [...(grp?.querySelectorAll('input[type=checkbox]') ?? [])];
    const label0 = boxes[0]?.closest('label')?.textContent ?? '';
    boxes[0]?.click();
    const chips = await wait(() => document.querySelector('.fa-chips'), 8000);
    const facetCount = Number((boxes[0]?.closest('label')?.querySelector('.fa-count')?.textContent ?? '').replace(/\D/g, ''));
    await wait(() => countNow() === facetCount, 10000);
    await sleep(300); await snap('class-filter');
    const total1 = countNow();
    await log('class-chip-and-filter', !!cchip && chipText.length > 0 && !/^Display/.test(chipText) && boxes.length > 0 && !!chips && total1 > 0 && total1 === facetCount && (boxes.length < 2 || total1 < total0), { facetCount, chipText, facets: boxes.length, label0, total0, total1, chips: chips?.textContent });
    chips?.querySelector('button')?.click();
    await wait(() => !document.querySelector('.fa-chips'), 5000);
    await log('class-filter-removed', countNow() === total0, { total0, now: countNow() });
    // Trùng lặp: mục sidebar khớp duplicate_summary thật; vào mục thấy nhóm "Cùng tệp", chỉ có hành động xem
    const dq = { text: null, vi: null, license: [], variable: null, italic: null, sourceIds: [], favoritesOnly: false, collectionId: null, tagIds: [], includeMissing: false, sort: 'family', descending: false, offset: 0, limit: 100, axisFilters: [], rawAxisRanges: [], hard: [], faceKeys: null, classes: [], subclasses: [], traits: [] };
    const dsum = await inv('duplicate_summary', { q: dq });
    const dupBtn = () => [...document.querySelectorAll('[aria-label="Thanh bên"] button')].find(x => /Trùng lặp/.test(x.textContent));
    await wait(() => (dsum.total > 0) === !!dupBtn(), 8000);
    await log('duplicates-sidebar-item', (dsum.total > 0) === !!dupBtn() && (dsum.total === 0 || Number((dupBtn().textContent.match(/(\d+)\s*$/) || [])[1]) === dsum.total), { dsum, item: dupBtn()?.textContent ?? null });
    if (dsum.total > 0) {
      dupBtn().click();
      await wait(() => document.querySelector('.dup [data-testid=dup-group]'), 20000);
      await sleep(400); await snap('duplicates-view');
      const dgroups = document.querySelectorAll('[data-testid=dup-group]').length;
      const dmembers = document.querySelectorAll('[data-testid=dup-member]').length;
      const dbtn = [...document.querySelectorAll('.dup button')].map(x => x.textContent.trim());
      const risky = dbtn.filter(x => /xoá|xóa|delete|di chuyển|move|gộp/i.test(x));
      const openBtn = [...document.querySelectorAll('.dup button')].find(x => x.textContent.trim() === 'Mở chi tiết');
      openBtn?.click();
      const panel = await wait(() => document.querySelector('aside.fa-panel h2'), 15000);
      await log('duplicates-view', dgroups === dsum.total && dmembers >= 2 && risky.length === 0 && !!panel && !/Lỗi giao diện/.test(document.body.textContent ?? ''), { dgroups, dmembers, risky, panel: panel?.textContent });
    } else {
      await log('duplicates-view', true, 'không có nhóm trùng trong fixture QA: mục sidebar ẩn đúng');
    }
    await log('done', true, '');
  } catch (e) { await log('error', false, String(e && e.stack || e)); }
})();
