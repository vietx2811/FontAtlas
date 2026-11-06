(async () => {
  const T = window.__TAURI_INTERNALS__;
  const inv = (c, a) => T.invoke(c, a);
  let n = 0;
  const log = async (step, ok, detail) => { n++; try { await inv('set_setting', { key: 'ui.qa.' + String(n).padStart(2, '0'), value: JSON.stringify({ step, ok, detail }) }); } catch (e) {} };
  const sleep = ms => new Promise(r => setTimeout(r, ms));
  const wait = async (fn, ms = 20000) => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = fn(); if (v) return v; } catch (e) {} await sleep(150); } return null; };
  const btn = (text, root = document) => [...root.querySelectorAll('button')].find(b => b.textContent.trim() === text && !b.disabled);
  const snap = async name => { try { await inv('plugin:event|emit', { event: 'qa-snap', payload: name }); } catch (e) {} await sleep(900); };
  const btnHas = (text, root = document) => [...root.querySelectorAll('button')].find(b => b.textContent.includes(text) && !b.disabled);
  try {
    // Chặn hộp thoại chọn thư mục gốc: trả đường dẫn QA (luồng React/IPC còn lại là thật).
    const picks = []; window.__FA_QA_PICKS__ = picks;
    await log('dialog-stub', true, 'window.__FA_QA_PICKS__');
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
    await log('view-family', !!chip, { chip: chip?.textContent, title: chip?.title, target, cells: cells().length, cellNames: cells().slice(0, 6).map(c => c.textContent.replace(/\s+/g, ' ').slice(-40)) });
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
    await log('filter-italic', !!fp && !!it, { inputs, ranges, before, after: cells().length, chip: chip2?.title, trigger: fbtn?.textContent });
    it?.click(); await sleep(1000);
    document.querySelector('[aria-label="Đóng bộ lọc"]')?.click();
    await log('filter-cleared', true, { cells: cells().length, chip: document.querySelector('.fa-family-chip')?.title });
    // CÀI ĐẶT → SIDEBAR
    document.querySelector('button[aria-label="Cài đặt"]')?.click();
    const st = await wait(() => document.querySelector('[data-testid=settings]'));
    await snap('settings');
    const nav = [...document.querySelectorAll('[aria-label="Thanh bên"] button, nav button, aside button')].find(b => b.textContent.includes('Tất cả'));
    nav?.click();
    const left = await wait(() => !document.querySelector('[data-testid=settings]') && cells().length > 0, 10000);
    await sleep(800); await snap('after-settings');
    await log('settings-sidebar-leaves', !!st && !!left, { settingsOpened: !!st, navFound: !!nav, cells: cells().length });
    await log('done', true, '');
  } catch (e) { await log('error', false, String(e && e.stack || e)); }
})();
