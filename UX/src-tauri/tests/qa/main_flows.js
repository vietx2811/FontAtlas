(async () => {
  const T = window.__TAURI_INTERNALS__;
  const inv = (c, a) => T.invoke(c, a);
  let n = 0; const T0 = Date.now();
  const log = async (step, ok, detail) => { n++; try { await inv('set_setting', { key: 'ui.qa.' + String(n).padStart(2, '0'), value: JSON.stringify({ step, ok, detail, t: Math.round((Date.now() - T0) / 1000) }) }); } catch (e) {} };
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const wait = async (fn, ms = 20000) => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch (e) {} await sleep(150); } return null; };
  const btn = (text, root = document) => [...root.querySelectorAll('button')].find(b => b.textContent.trim() === text && !b.disabled);
  // Cửa sổ QA có thể không ở trước nên WKWebView không chạy animation/transition: chốt về trạng thái cuối trước khi chụp.
  const settleAnims = () => { try { document.getAnimations().forEach(x => { try { x.finish(); } catch (e) {} }); } catch (e) {} };
  const snap = async name => { settleAnims(); await sleep(80); try { await inv('plugin:event|emit', { event: 'qa-snap', payload: name }); } catch (e) {} await sleep(350); };
  const btnHas = (text, root = document) => [...root.querySelectorAll('button')].find(b => b.textContent.includes(text) && !b.disabled);
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
    const it = fp && [...fp.querySelectorAll('label')].find(l => l.textContent.includes('In nghiêng'))?.querySelector('input');
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
    await log('done', true, '');
  } catch (e) { await log('error', false, String(e && e.stack || e)); }
})();
