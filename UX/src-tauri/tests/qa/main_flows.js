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
  // Ngôn ngữ giao diện cố định cho cả lượt QA (mặc định vi; QA_LANG=en cho lượt chụp ảnh tiếng Anh). Không phụ thuộc ngôn ngữ của máy.
  await wait(() => window.__faLocale, 20000); window.__faLocale?.('__QA_LANG__'); await sleep(500);
  // Bộ lọc nay là sheet Advanced (nút data-testid=advanced-open); đóng bằng nút × của sheet.
  const openAdv = async () => { if (!document.querySelector('[data-testid=advanced-sheet]')) document.querySelector('[data-testid=advanced-open]')?.click(); return await wait(() => document.querySelector('[data-testid=advanced-sheet]'), 8000); };
  const closeAdv = async () => { document.querySelector('.fa-adv-close')?.click(); await wait(() => !document.querySelector('[data-testid=advanced-sheet]'), 5000); };
  const axisPairs = () => [...document.querySelectorAll('[data-testid=advanced-sheet] [data-testid=axis-filter-row]')].map(r => [...r.querySelectorAll('.fa-axrow-nums input')].map(i => i.value).join('–')).filter(v => v.includes('–'));
  const btn = (text, root = document) => [...root.querySelectorAll('button')].find(b => b.textContent.trim() === text && !b.disabled);
  // Cửa sổ QA có thể không ở trước nên WKWebView không chạy animation/transition: chốt về trạng thái cuối trước khi chụp.
  const settleAnims = () => { try { document.getAnimations().forEach(x => { try { x.finish(); } catch (e) {} }); } catch (e) {} };
  const snap = async name => { settleAnims(); await sleep(80); try { await inv('plugin:event|emit', { event: 'qa-snap', payload: name }); } catch (e) {} await sleep(350); };
  const btnHas = (text, root = document) => [...root.querySelectorAll('button')].find(b => b.textContent.includes(text) && !b.disabled);
  // Ảnh PNG xuất ra tệp qua bảng settings (giới hạn 64 KB/khoá → cắt từng đoạn ui.qapng.<theme>.NN); run.sh ghép lại.
  const savePng = async (th, url) => { const errs = []; if (!url) return ['không có dataUrl']; const b = url.split(',')[1]; for (let i = 0, n = 0; i < b.length; i += 60000, n++) { try { await inv('set_setting', { key: 'ui.qapng.' + th + '.' + String(n).padStart(3, '0'), value: b.slice(i, i + 60000) }); } catch (e) { errs.push('đoạn ' + n + ': ' + String(e && e.message || e).slice(0, 120)); } } return errs; };
  // Thuật ngữ typo: rê chuột vào từng nhãn [data-term], tooltip phải có gloss không rỗng (và hình minh hoạ nếu thuật ngữ có hình).
  // Tất định: không đoán độ trễ. Chờ đúng thẻ của đúng thuật ngữ (data-term-card = data-term) cho tới khi nó xuất hiện (trần 20 s chỉ để không treo
  // khi máy quá tải; tooltip hiện sau 450 ms nhưng timer bị trì hoãn khi máy bận), rồi chờ tooltip đóng hẳn trước khi sang thuật ngữ kế.
  // Không nới điều kiện: vẫn yêu cầu gloss không rỗng; thẻ của thuật ngữ KHÁC (tooltip trước chưa đóng) không được tính.
  const termCheck = async sel => {
    const terms = [...new Set([...document.querySelectorAll(sel)].map(e => e.dataset.term))]; const bad = [], artMissing = []; let art = 0; const n = terms.length;
    const card = term => [...document.querySelectorAll('[role=tooltip] [data-term-card]')].find(c => c.dataset.termCard === term);
    // Phần tử tra lại SỐNG mỗi lần (màn có thể vẽ lại khi dữ liệu về muộn): sự kiện gửi vào nút đã bị gỡ khỏi DOM sẽ không tới React.
    const live = term => [...document.querySelectorAll(sel)].find(e => e.dataset.term === term);
    const fire = (el, type) => el.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerType: 'mouse' }));
    for (const term of terms) {
      let el = live(term);
      if (!el) { bad.push(term + ' (nhãn biến mất)'); continue; }
      fire(el, 'pointerover');
      const c = await wait(() => { const x = card(term); return x && x.querySelector('.fa-term-gloss')?.textContent.trim() ? x : null; }, 20000);
      if (!c) bad.push(term + ' [giai đoạn: mở — thẻ/chú giải không hiện]');
      else {
        // Hình minh hoạ vẽ/nạp muộn trên máy tải: đợi tối đa 3 s trước khi đo (thiếu hình sau 3 s vẫn tính là không có hình).
        const withArt = await wait(() => card(term)?.querySelector('svg.fa-term-art'), 3000);
        if (withArt) art++; else artMissing.push(term);
      }
      el = live(term) ?? el; fire(el, 'pointerout');
      const closed = await wait(() => !document.querySelector('[role=tooltip]'), 2000);
      if (!closed) bad.push(term + ' [giai đoạn: đóng — tooltip không đóng sau 2 s; còn thẻ: ' + [...document.querySelectorAll('[role=tooltip] [data-term-card]')].map(x => x.dataset.termCard).join(',') + '; nhãn còn trong DOM: ' + !!live(term) + '; tooltip: ' + [...document.querySelectorAll('[role=tooltip]')].map(x => (x.id || '') + '=' + (x.textContent || '').slice(0, 40) + ' desc-by:' + ([...document.querySelectorAll('[aria-describedby]')].filter(o => (o.getAttribute('aria-describedby') || '').includes(x.id)).map(o => o.tagName + '.' + o.className + '[' + (o.dataset.term || '') + ']').join('|'))).join(' ; ') + '; focus: ' + (document.activeElement?.tagName ?? '') + '.' + (document.activeElement?.className ?? '') + '; splash/inert: ' + !!document.querySelector('[data-testid=splash],[inert]') + ')');
    }
    return { n, bad, art, artMissing };
  };
  try {
    // Chặn hộp thoại chọn thư mục gốc: trả đường dẫn QA (luồng React/IPC còn lại là thật).
    const picks = []; window.__FA_QA_PICKS__ = picks;
    await log('dialog-stub', true, 'window.__FA_QA_PICKS__');
    // Script chạy ngay khi trang load xong: chờ React vẽ onboarding hoặc thư viện (máy bận có thể > 10 s).
    // SPLASH (lớp phủ khi mở app, ~2 s): phím bỏ qua được và bị NUỐT — Esc không được rơi xuống onboarding (O1 có "Bỏ qua hướng dẫn"), phần dưới inert.
    // Nếu máy chậm và splash đã tự kết thúc thì không có gì để kiểm.
    const splash = document.querySelector('[data-testid=splash]');
    if (splash) {
      const inertBelow = !!document.querySelector('[inert]');
      const esc = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
      document.dispatchEvent(esc);
      await wait(() => !document.querySelector('[data-testid=splash]'), 4000);
      // Esc phải bị nuốt (defaultPrevented) — listener ở window capture + layout effect nên không có khoảng hở khi máy tải; Esc không rơi xuống onboarding còn kiểm ở splash-esc-stays-o1.
      await log('splash-skip', !document.querySelector('[data-testid=splash]') && inertBelow && esc.defaultPrevented, { inertBelow, prevented: esc.defaultPrevented });
      await wait(() => document.querySelector('.fa-onb h1, .fa-lib'), 90000);
      await sleep(400);
      const h1 = document.querySelector('.fa-onb h1')?.textContent ?? null;
      // Lần đầu: vẫn đứng ở bước 1 (không bị Esc bỏ cả hướng dẫn). Không phải lần đầu: không có onboarding thì không có gì để kiểm.
      await log('splash-esc-stays-o1', h1 === null ? !!document.querySelector('.fa-lib') : /Chào mừng/.test(h1), { h1 });
    } else await log('splash-skip', true, 'đã tự kết thúc trước khi kiểm');
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
      if (h1 === 'Xong' && !document.querySelector('[role=dialog]')) {
        // Cuối hướng dẫn: bảng phím tắt hiện TRƯỚC outro (lưới thẻ, bỏ qua được); ô minh hoạ vuông không viền.
        const vb = document.querySelector('.fa-stage svg.fa-bp')?.getAttribute('viewBox');
        const bw = document.querySelector('.fa-stage svg.fa-bp') ? getComputedStyle(document.querySelector('.fa-stage svg.fa-bp')).borderTopWidth : null;
        btn('Vào thư viện')?.click();
        const dlg = await wait(() => document.querySelector('[role=dialog] .fa-tips-grid'), 8000);
        await sleep(300); await snap('onb-tips');
        const cards = dlg ? dlg.querySelectorAll('[data-testid=tip-card]').length : 0;
        const dots = document.querySelectorAll('.fa-onb-dots i').length;
        await log('onb-tips-overlay', !!dlg && cards >= 8 && vb === '0 0 480 480' && bw === '0px' && dots >= 6 && !document.querySelector('.fa-onb-keys, .fa-onb-stepno, .fa-legend'), { cards, vb, bw, dots, legend: !!document.querySelector('.fa-legend') });
        continue;
      }
      const order = /Kích hoạt/.test(h1) ? ['Để sau', 'Tiếp tục'] : ['Đã hiểu', 'Không dùng', 'Bỏ qua', 'Tiếp tục', 'Bắt đầu', 'Vào thư viện'];
      // Intro (nút "Bỏ qua") và outro (~1,5 s, không có nút) là màn chuyển động: outro kết thúc thì .fa-onb biến mất, thoát vòng lặp.
      const b = await wait(() => (document.querySelector('.fa-onb') ? order.map(t => btn(t)).find(Boolean) : true), 120000);
      if (!b) { await log('onb-stuck', false, h1); break; }
      if (b === true) break;
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
    const numeric = texts.filter(t => /\d+[.,]\d+\s*(em|°|×)|\d+\s*°/.test(t));
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
    const fbtn = document.querySelector('[data-testid=advanced-open]');
    // Phễu Advanced nằm trong ô tìm kiếm: nút icon (không chữ), có aria-label; không còn nút "Advanced" riêng trên Top Bar.
    await log('advanced-funnel-in-search', !!fbtn && !!fbtn.closest('.fa-search') && (fbtn.textContent.trim() === '' || !!fbtn.querySelector('.fa-funnel-n')) && !!fbtn.getAttribute('aria-label') && !btn('Advanced', document.querySelector('.fa-topbar')), { inSearch: !!fbtn?.closest('.fa-search'), text: fbtn?.textContent, label: fbtn?.getAttribute('aria-label') });
    const fp = await openAdv();
    await wait(() => document.querySelectorAll('[data-testid=advanced-sheet] [data-testid=axis-filter-row]').length === 18, 20000);
    const inputs = fp ? [...fp.querySelectorAll('.fa-axrow-nums input')].slice(0, 4).map(i => i.value) : [];
    const ranges = axisPairs().slice(0, 3);
    const before = cells().length;
    const it = fp && [...fp.querySelectorAll('label')].find(l => (l.textContent.includes('Italic') || l.textContent.includes('In nghiêng')))?.querySelector('input');
    it?.click();
    await sleep(1500);
    const chip2 = document.querySelector('.fa-family-chip');
    await snap('filter-italic');
    await log('filter-italic', !!fp && !!it, { inputs, ranges, before, after: cells().length, chip: chip2?.getAttribute('aria-label'), trigger: fbtn?.textContent });
    it?.click(); await sleep(1000);
    await closeAdv();
    await log('filter-cleared', true, { cells: cells().length, chip: document.querySelector('.fa-family-chip')?.getAttribute('aria-label') });
    // BỘ LỌC trên toàn thư viện: khoảng slider lấy p1–p99 của chính thư viện, không phải 0–1
    document.querySelector('.fa-family-chip')?.click(); await sleep(1000);
    const fpAll = await openAdv();
    await wait(() => axisPairs().length >= 6, 20000);
    await sleep(800); await snap('filter-full-library');
    const rangesAll = axisPairs().slice(0, 6);
    await log('filter-full-library', !!fpAll && rangesAll.length >= 3 && rangesAll.some(r => r !== '0–1') && !document.querySelector('.fa-family-chip'), { cells: cells().length, rangesAll });
    await closeAdv(); await sleep(300);
    // ADVANCED + TÓM TẮT + "ĐANG LỌC" (2a)
    {
      const key = (el, init) => el.dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init }));
      const total = () => Number((document.querySelector('[data-testid=view-count]')?.textContent ?? '').replace(/\D/g, '')) || 0;
      document.querySelector('.fa-family-chip')?.click(); await sleep(600);
      const base = total();
      key(document, { key: 'f', code: 'KeyF', metaKey: true, altKey: true });
      const sheet = await wait(() => document.querySelector('[data-testid=advanced-sheet]'), 8000);
      const rows = await wait(() => { const r = document.querySelectorAll('[data-testid=advanced-sheet] [data-testid=axis-filter-row]'); return r.length === 18 ? r : null; }, 20000);
      await sleep(600);
      const th0 = document.documentElement.dataset.theme;
      for (const th of ['light', 'dark']) { document.documentElement.dataset.theme = th; await sleep(250); await snap('advanced-sheet-' + th); }
      if (th0) document.documentElement.dataset.theme = th0; else delete document.documentElement.dataset.theme;
      await sleep(150);
      const cols = sheet && getComputedStyle(sheet.querySelector('[data-testid=advanced-cols]'));
      const cards = sheet ? [...sheet.querySelectorAll('.fa-adv-card')] : [];
      const lefts = new Set(cards.map(c => Math.round(c.getBoundingClientRect().left)));
      const scrollNeeded = sheet ? sheet.querySelector('[data-testid=advanced-cols]').scrollHeight > sheet.querySelector('[data-testid=advanced-cols]').clientHeight + 1 : null;
      await log('advanced-opens-18-axes', !!sheet && !!rows && lefts.size >= 2 && new Set(cards.filter(c => c.previousElementSibling === null).map(c => Math.round(c.getBoundingClientRect().top))).size === 1, { rows: rows?.length ?? 0, lanes: sheet?.querySelector('[data-testid=advanced-cols]')?.dataset.cols, distinctColumns: lefts.size, laneTops: [...new Set(cards.filter(c => c.previousElementSibling === null).map(c => Math.round(c.getBoundingClientRect().top)))], scrollNeeded, win: [innerWidth, innerHeight] });
      const tt = await termCheck('[data-testid=advanced-sheet] .fa-axis-name[data-term]');
      await log('advanced-term-tooltips', tt.n >= 17 && tt.bad.length === 0 && tt.art > 0, tt);
      // Weight 600+ bằng bàn phím: lưới đổi số lượng, tóm tắt hiện Weight, nháy đúp thanh = bỏ lọc
      const lo = document.querySelector('[data-axis=weight_class] [data-testid=rs-lo]');
      lo?.focus(); for (let i = 0; i < 5; i++) { key(lo, { key: 'ArrowRight' }); await sleep(120); }
      await sleep(900);
      const sum = document.querySelector('[data-testid=filter-summary]');
      const after = total();
      await log('advanced-weight-filters-grid', !!sum && /Weight/.test(sum.textContent) && after > 0 && after < base, { base, after, summary: sum?.textContent });
      document.querySelector('[data-axis=weight_class] [data-testid=rs-track]')?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }));
      await sleep(900);
      await log('advanced-weight-dblclick-resets', !document.querySelector('[data-testid=filter-summary]') && total() === base, { total: total(), base });
      key(document, { key: 'Escape' }); await sleep(300);
      await log('advanced-esc-closes', !document.querySelector('[data-testid=advanced-sheet]'), null);
      // LỌC NGÔN NGỮ (Hyperglot): chọn Tiếng Việt "đủ" → lưới còn font đủ dấu, mọi badge là vie/full; ảnh picker (dark)
      {
        const fpL = await openAdv();
        const row = await wait(() => fpL?.querySelector('[data-code=vie]'), 30000);
        const totalBefore = total();
        row?.click();
        await wait(() => document.querySelector('[data-testid=filter-summary]')?.textContent.includes('Tiếng Việt') || document.querySelector('[data-testid=filter-summary] [data-group=lang]'), 8000);
        await sleep(1200);
        const labelFull = row?.getAttribute('aria-label') ?? '';
        const th2 = document.documentElement.dataset.theme;
        document.documentElement.dataset.theme = 'dark'; await sleep(250);
        await snap('lang-picker-dark');
        if (th2) document.documentElement.dataset.theme = th2; else delete document.documentElement.dataset.theme;
        await closeAdv(); await sleep(500);
        const marks = [...document.querySelectorAll('[role=gridcell] [data-testid=vi-mark]')];
        const after = total();
        const bad = marks.filter(m => !(m.dataset.lang === 'vie' && m.dataset.state === 'full')).map(m => m.dataset.state);
        await log('lang-filter-narrows-and-badges', !!row && /(đủ|full)$/.test(labelFull) && after > 0 && after <= totalBefore && marks.length > 0 && bad.length === 0, { totalBefore, after, label: labelFull, marks: marks.length, bad });
        await openAdv();
        // Bấm tới khi dòng về "chưa chọn" (full → near → off); tra lại phần tử mỗi lần vì danh sách vẽ lại khi dữ liệu về.
        for (let i = 0; i < 4; i++) { const r2 = await wait(() => document.querySelector('[data-code=vie]'), 10000); if (/(chưa chọn|not selected)$/.test(r2?.getAttribute('aria-label') ?? '')) break; r2?.click(); await sleep(700); }
        await closeAdv(); await sleep(400);
        // Số đếm cập nhật bất đồng bộ sau khi bỏ nhóm lọc: đợi tới khi về đúng giá trị (tối đa 8 giây) thay vì đọc ngay sau một lần sleep cố định.
        const langCleared = await wait(() => !document.querySelector('[data-testid=filter-summary] [data-group=lang]') && total() === totalBefore, 8000);
        await log('lang-filter-cleared', !!langCleared, { total: total(), totalBefore });
      }
      // Tóm tắt trong ô tìm + Backspace hai lần + Hoàn tác
      const fp2 = await openAdv();
      [...(fp2?.querySelectorAll('label') ?? [])].find(l => l.textContent.includes('Italic') || l.textContent.includes('In nghiêng'))?.querySelector('input')?.click();
      await sleep(900);
      await closeAdv(); await sleep(300);
      const sbox = document.querySelector('.fa-search'); const sinput = sbox?.querySelector('input');
      const sum2 = await wait(() => document.querySelector('[data-testid=filter-summary]'), 8000);
      const ratio = sinput && sbox ? sinput.getBoundingClientRect().width / sbox.getBoundingClientRect().width : 0;
      await log('summary-keeps-input-wide', !!sum2 && ratio >= 0.4, { ratio, summary: sum2?.textContent });
      sum2?.click();
      const pop = await wait(() => document.querySelector('[data-testid=active-filters]'), 5000);
      await sleep(300);
      const th1 = document.documentElement.dataset.theme;
      for (const th of ['light', 'dark']) { document.documentElement.dataset.theme = th; await sleep(250); await snap('active-filters-popover-' + th); }
      if (th1) document.documentElement.dataset.theme = th1; else delete document.documentElement.dataset.theme;
      await log('active-filters-popover', !!pop && pop.querySelectorAll('.fa-fpop-row').length >= 1 && /(Xoá tất cả|Clear all)/.test(pop.textContent), { rows: pop?.querySelectorAll('.fa-fpop-row').length, text: pop?.textContent.slice(0, 80) });
      key(document, { key: 'Escape' }); await sleep(300);
      const withFilter = total();
      sinput?.focus(); key(sinput, { key: 'Backspace' });
      const selected = !!(await wait(() => document.querySelector('.fa-fsum-seg[data-selected=true]'), 5000)); // chọn nhóm cuối: đợi trạng thái thay vì sleep cố định
      key(sinput, { key: 'Backspace' });
      await wait(() => !document.querySelector('[data-testid=filter-summary]'), 5000);
      const undoBtn = await wait(() => [...document.querySelectorAll('.fa-toast button')].find(b => /^(Hoàn tác|Undo)$/.test(b.textContent.trim())), 5000);
      await log('backspace-drops-last-group', selected && !document.querySelector('[data-testid=filter-summary]') && !!undoBtn, { selected, left: !!document.querySelector('[data-testid=filter-summary]'), undo: !!undoBtn });
      undoBtn?.click(); await sleep(900);
      await log('toast-undo-restores', !!document.querySelector('[data-testid=filter-summary]') && total() === withFilter, { total: total(), withFilter });
      // dọn: bỏ lọc để các bước sau chạy trên thư viện đầy đủ
      key(sinput ?? document, { key: 'Backspace', metaKey: true }); await sleep(600);
      await log('summary-clear-all', !document.querySelector('[data-testid=filter-summary]'), null);
    }
    // CÀI ĐẶT → SIDEBAR
    document.querySelector('button[aria-label="Cài đặt"]')?.click();
    const st = await wait(() => document.querySelector('[data-testid=settings]'));
    await snap('settings');
    // Theme tối qua radio trong Cài đặt (đường thật: applyTheme + theme cửa sổ native cho lớp kính sidebar), không đặt data-theme bằng JS.
    {
      const radioOf = re => [...document.querySelectorAll('[data-testid=settings] input[type=radio][name=theme]')].find(r => re.test(r.closest('label')?.textContent ?? ''));
      radioOf(/^(Tối|Dark)$/)?.click(); await sleep(900);
      await snap('settings-dark-real');
      const darkApplied = document.documentElement.dataset.theme === 'dark';
      radioOf(/^(Sáng|Light)$/)?.click(); await sleep(700);
      await log('settings-theme-radio-dark', darkApplied && document.documentElement.dataset.theme === 'light', { darkApplied, after: document.documentElement.dataset.theme });
    }
    // Bật font tự động (0.3.0): nhóm có đủ 6 app theo thứ tự cố định, công tắc chung, nhật ký; trạng thái thật do backend quyết nên chỉ kiểm cấu trúc.
    {
      [...document.querySelectorAll('.fa-set-nav button')].find(b => /(Bật font tự động|Auto-activation)/.test(b.textContent))?.click();
      await wait(() => document.querySelectorAll('[data-testid=auto-app]').length === 6, 15000);
      const ids = [...document.querySelectorAll('[data-testid=auto-app]')].map(x => x.dataset.app).join(',');
      await sleep(300);
      for (const th of ['dark', 'light']) { document.documentElement.dataset.theme = th; await sleep(250); await snap('auto-activation-' + th); }
      await log('auto-activation-page', ids === 'photoshop,aftereffects,illustrator,indesign,incopy,affinity' && !!document.querySelector('[data-testid=auto-switch]') && !!document.querySelector('[data-testid=auto-log], [data-testid=auto-log-empty]') && !document.querySelector('[data-testid=auto-error]'), { ids, error: document.querySelector('[data-testid=auto-error]')?.textContent ?? null });
    }
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
    // Tất định: tooltip phải là của ĐÚNG phần tử (aria-describedby trỏ tới nó), và phải đóng hẳn trước khi sang phần tử kế; không dùng sleep cố định.
    const tipFor = el => () => { const x = tip(); return x && el.getAttribute('aria-describedby')?.split(' ').includes(x.id) ? x : null; };
    const closed = () => wait(() => !tip(), 5000);
    const t1 = await wait(tipFor(vi0), 20000);
    const t1Text = t1?.textContent; await sleep(100); await snap('tooltip-badge');
    unhover(vi0); await closed();
    const gear = document.querySelector('button[aria-label="Cài đặt"]');
    hover(gear);
    const t2 = await wait(tipFor(gear), 20000);
    const t2Text = t2?.textContent; await sleep(100); await snap('tooltip-settings');
    unhover(gear); await closed();
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

    // ACTIVE (2c): công tắc có nhãn, trạng thái + phạm vi, nút trên thẻ, Top Bar, ⌘⇧A, cả family. Engine CoreText THẬT nhưng chỉ font fixture trong thư mục QA;
    // mọi bước trả về trạng thái "tắt" và cuối cùng kiểm tra không còn font nào bật (phạm vi Persistent chỉ bật thoáng qua rồi đổi lại).
    {
      const actOn = async () => (await inv('activation_state', {})).filter(x => x.state === 'active');
      const waitActive = async (pred, ms = 15000) => { const t0 = Date.now(); let last = null; while (Date.now() - t0 < ms) { last = await actOn(); if (pred(last)) return last; await sleep(200); } return last; };
      const row = () => document.querySelector('aside.fa-panel [data-testid=active-row]');
      const sw = () => row()?.querySelector('[data-testid=active-switch]');
      const status = () => row()?.querySelector('[data-testid=active-status]')?.textContent;
      // Số trên mục "Đang bật" của thanh bên phải đi theo mỗi lần bật/tắt (trước đây đứng yên ở 0).
      const navCount = () => [...document.querySelectorAll('button')].find(b => /^Đang bật/.test(b.textContent ?? ''))?.textContent?.replace(/\D/g, '');
      await wait(row, 10000);
      const termA2 = await termCheck('[data-testid=active-row] [data-term]');
      await log('active-peek-labelled', !!sw() && sw().getAttribute('aria-label') === 'Active' && sw().getAttribute('aria-checked') === 'false' && status() === 'Chưa bật' && termA2.n === 1 && termA2.bad.length === 0 && termA2.art === 1, { aria: sw()?.getAttribute('aria-label'), checked: sw()?.getAttribute('aria-checked'), status: status(), termA2 });
      sw().click();
      await wait(() => status() === 'Đang bật · phiên này', 15000);
      let st = await waitActive(x => x.length === 1);
      await wait(() => navCount() === '1', 8000);
      await snap('active-peek-on');
      await log('active-sidebar-count', navCount() === '1', { nav: navCount() });
      await log('active-peek-on-session', status() === 'Đang bật · phiên này' && sw().getAttribute('aria-checked') === 'true' && st.length === 1 && st[0].scope === 'session', { status: status(), state: st.map(x => [x.hashHex.slice(0, 8), x.scope, x.status]) });
      const scopeBtn = () => row()?.querySelector('[data-testid=active-scope]');
      scopeBtn()?.click();
      await wait(() => status() === 'Luôn bật', 15000);
      st = await waitActive(x => x.length === 1 && x[0].scope === 'persistent');
      const persistOk = status() === 'Luôn bật' && st.length === 1 && st[0].scope === 'persistent' && sw().getAttribute('aria-checked') === 'true';
      scopeBtn()?.click();
      await wait(() => status() === 'Đang bật · phiên này', 15000);
      st = await waitActive(x => x.length === 1 && x[0].scope === 'session');
      await log('active-peek-scope', persistOk && status() === 'Đang bật · phiên này' && st.length === 1 && st[0].scope === 'session', { persistOk, status: status(), state: st.map(x => [x.scope, x.status]) });
      // Thẻ: font đang mở trong panel là font vừa bật (cmd-click có thể đưa panel sang thẻ thứ hai): tìm theo trạng thái, không theo chỉ số.
      const pressed = c => c.querySelector('[data-testid=cell-active]')?.getAttribute('aria-pressed') === 'true';
      const [cOn, cOff] = [cells()[0], cells()[1]].sort((x, y) => Number(pressed(y)) - Number(pressed(x)));
      const dot = cOn.querySelector('[data-testid=active-dot]');
      const powBtn = cOff.querySelector('[data-testid=cell-active]');
      // Chấm là nút bật/tắt, đứng ĐẦU tên: xanh khi bật, xám khi tắt; badge ngôn ngữ không dùng màu xanh của chấm.
      const firstOf = c => c.querySelector('.fa-meta')?.firstElementChild;
      const bgOf = c => getComputedStyle(c.querySelector('.fa-dot')).backgroundColor;
      const badge = cOn.querySelector('.fa-lang-badge');
      await log('active-dot-first-and-badge-neutral', firstOf(cOn) === cOn.querySelector('[data-testid=cell-active]') && firstOf(cOff) === powBtn && powBtn.tagName === 'BUTTON' && bgOf(cOn) !== bgOf(cOff) && (!badge || getComputedStyle(badge).color !== bgOf(cOn)), { on: bgOf(cOn), off: bgOf(cOff), badge: badge && getComputedStyle(badge).color });
      await log('active-card-controls', pressed(cOn) && !pressed(cOff) && !!dot && !cOff.querySelector('[data-testid=active-dot]') && !!powBtn, { onHasDot: !!dot, offPressed: pressed(cOff), onPressed: pressed(cOn) });
      powBtn.click();
      st = await waitActive(x => x.length === 2);
      await wait(() => cOff.querySelector('[data-testid=active-dot]'), 8000);
      await snap('active-cards-on');
      await log('active-card-toggle', st.length === 2 && !!cOff.querySelector('[data-testid=active-dot]') && /^2 đã chọn$/.test(document.querySelector('[data-testid=selection-count]')?.textContent ?? ''), { n: st.length, selection: document.querySelector('[data-testid=selection-count]')?.textContent });
      // Thanh chọn: cả lựa chọn đang bật → nút "Tắt kích hoạt"; bấm → tắt hết; "Kích hoạt" → bật hết; ⌘⇧A → tắt.
      // Top Bar KHÔNG còn nút nguồn (hai nút cùng việc chỉ làm rối); ⌘⇧A vẫn chạy.
      const sbBtn = () => document.querySelector('[data-testid=selbar-activate]');
      const lbl = () => sbBtn()?.textContent;
      const noTopBtn = !document.querySelector('header.fa-topbar [data-testid=topbar-activate]');
      const labelOk1 = lbl() === 'Tắt kích hoạt';
      sbBtn().click();
      st = await waitActive(x => x.length === 0); await wait(() => lbl() === 'Kích hoạt', 8000);
      const offOk = st.length === 0 && lbl() === 'Kích hoạt' && navCount() === '0';
      sbBtn().click();
      st = await waitActive(x => x.length === 2); await wait(() => lbl() === 'Tắt kích hoạt', 8000);
      await wait(() => navCount() === '2', 8000);
      const onOk = st.length === 2 && lbl() === 'Tắt kích hoạt' && navCount() === '2';
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'A', metaKey: true, shiftKey: true, bubbles: true, cancelable: true }));
      st = await waitActive(x => x.length === 0);
      await wait(() => navCount() === '0', 8000);
      await log('active-topbar-multi', noTopBtn && labelOk1 && offOk && onOk && st.length === 0 && navCount() === '0', { noTopBtn, labelOk1, offOk, onOk, afterShortcut: st.length, nav: navCount() });
      // Cả family: nút cạnh StyleDropdown bật mọi kiểu của family (≥ 2 file), bấm lại tắt hết.
      const famBtn = () => document.querySelector('aside.fa-panel [data-testid=family-active]');
      const styleCount = Number((document.querySelector('aside.fa-panel .fa-dd-btn .cnt')?.textContent ?? '').replace(/\D/g, '')) || 0;
      famBtn()?.click();
      st = await waitActive(x => x.length >= 1 && (styleCount <= 1 || x.length >= 2) && famBtn()?.getAttribute('aria-pressed') === 'true');
      const famOn = !!famBtn() && famBtn().getAttribute('aria-pressed') === 'true';
      await snap('active-family-on');
      famBtn()?.click();
      const stOff = await waitActive(x => x.length === 0);
      await wait(() => famBtn()?.getAttribute('aria-pressed') === 'false', 8000);
      await log('active-family', famOn && stOff.length === 0 && famBtn()?.getAttribute('aria-pressed') === 'false', { styleCount, onCount: st.length, famOn, offCount: stOff.length });
      // Dark: ảnh panel + thẻ đang bật.
      sw().click(); await waitActive(x => x.length === 1);
      const themeAct = document.documentElement.dataset.theme; document.documentElement.dataset.theme = 'dark'; await sleep(400);
      await snap('active-peek-dark');
      if (themeAct) document.documentElement.dataset.theme = themeAct; else document.documentElement.removeAttribute('data-theme'); await sleep(200);
      sw().click(); const fin = await waitActive(x => x.length === 0);
      await log('active-clean', fin.length === 0, { left: fin.map(x => [x.hashHex.slice(0, 8), x.scope]) });
    }

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
    await log('compare-edit-specimen', document.querySelectorAll('.cmp-edit').length === 2 && views.every(v => v === 'Quạ đen 龍') && document.querySelector('.cmp-edit').value === 'Quạ đen 龍' && !!missNote && missCol !== inkCol && specNotes().length === 2 && [...document.querySelectorAll('.cmp-spec .miss')].every(m => !/^[A-Za-z0-9 ]$/.test(m.textContent)) && [...document.querySelectorAll('.cmp-spec-view')].every(v => v.querySelector('.miss')?.textContent === '龍' || [...v.querySelectorAll('.miss')].some(m => m.textContent === '龍')), { views, missNote, missCol, inkCol, miss: document.querySelectorAll('.cmp-spec .miss').length });
    // Chẩn đoán phát hiện glyph thiếu trong WKWebView thật (ghi số liệu để chỉnh phép thử)
    try {
      const famRaw = getComputedStyle(document.querySelector('.cmp-spec-view')).fontFamily; const famName = famRaw.split(',')[0].trim().replace(/^["']|["']$/g, '');
      const cv = document.createElement('canvas'); cv.width = 80; cv.height = 64; const g = cv.getContext('2d');
      const stacks = { famSerif: `40px "${famName}", serif`, famSans: `40px "${famName}", sans-serif`, famMono: `40px "${famName}", monospace`, famOnly: `40px "${famName}"`, noneSerif: '40px "__none__", serif', noneSans: '40px "__none__", sans-serif', noneOnly: '40px "__none__"' };
      const diag = {};
      for (const ch of ['龍', 'ạ', 'Q']) {
        diag[ch] = {};
        for (const [k, f] of Object.entries(stacks)) { g.clearRect(0, 0, 80, 64); g.font = f; g.fillStyle = '#000'; g.fillText(ch, 4, 48); const d = g.getImageData(0, 0, 80, 64).data; let sum = 0, h = 0; for (let i = 3; i < d.length; i += 4) { sum += d[i]; h = (h * 31 + d[i]) | 0; } g.font = f.replace('40px', '64px'); diag[ch][k] = `${sum}/${h}/${g.measureText(ch).width.toFixed(1)}`; }
      }
      await log('compare-glyph-diag', true, { famRaw, famName, diag, fontsCheck: document.fonts.check(`40px "${famName}"`, '龍') });
    } catch (e) { await log('compare-glyph-diag', true, { err: String(e) }); }
    // PNG giữ ký tự thiếu: xám + chữ ghi chú (ảnh không có hover)
    const pngMiss = await (window.__faQaComparePng ? window.__faQaComparePng() : null);
    await log('compare-png-missing-note', !!pngMiss && pngMiss.texts.some(x => /^Thiếu: /.test(x)), { notes: pngMiss?.texts.filter(x => /Thiếu/.test(x)) });
    // Esc của bộ gõ (isComposing/229) trong ô specimen không đóng màn Compare
    const ed = document.querySelector('.cmp-edit'); ed.focus();
    ed.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', keyCode: 229, isComposing: true, bubbles: true, cancelable: true })); await sleep(300);
    await log('compare-esc-ime-stays-open', !!document.querySelector('.cmp-tbl') && document.activeElement === ed, { open: !!document.querySelector('.cmp-tbl') });
    ed.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true })); await sleep(200); // Esc thường trong ô: chỉ blur, màn vẫn mở
    await log('compare-esc-in-field-blurs', document.activeElement !== ed && !!document.querySelector('.cmp-tbl'), {});
    setVal(document.querySelector('.cmp-edit'), prevSample ?? 'Chữ Việt đẹp — Ấn tượng'); await sleep(300);
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
    const fillGaps = [...document.querySelectorAll('.cmp-tbl .cmp-page')].map(pg => Math.round(pg.parentElement.getBoundingClientRect().bottom - pg.getBoundingClientRect().bottom));
    const noteBg = (document.querySelector('.cmp-tbl .fa-note') ? getComputedStyle(document.querySelector('.cmp-tbl .fa-note')).backgroundColor : '').match(/\d+/g)?.map(Number) ?? [];
    await log('compare-dark-fill-and-note-red', fillGaps.length > 0 && fillGaps.every(g => g <= 1) && noteBg.length >= 3 && noteBg[0] > 200 && noteBg[0] > noteBg[1] * 1.5 && noteBg[0] > noteBg[2] * 1.5, { fillGaps, noteBg });
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
    await log('compare-dark-specimen', lightSpec > 700 && darkSpec >= 0 && darkSpec < 60 && /^rgb\(25[0-9]/.test(darkInk) && !!pngD && pngD.dark === true && pngD.specimenBg.toLowerCase() === '#09090b', { lightSpec, darkSpec, darkInk, pngDark: pngD?.dark, bg: pngD?.specimenBg });
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
    // Tổng ban đầu: đợi tới khi > 0 và đứng yên qua hai lần đọc (thư viện còn đang tải thì số chưa ổn định).
    let total0 = 0; for (let i = 0, prev = -1; i < 40; i++) { total0 = countNow(); if (total0 > 0 && total0 === prev) break; prev = total0; await sleep(300); }
    await openAdv();
    const grp = await wait(() => document.querySelector('[aria-label="Phân loại"]'), 10000);
    await wait(() => grp?.querySelectorAll('[data-testid^="class-facet-"]').length > 0, 30000); // facet tải sau debounce; máy chậm có thể lâu hơn 15 giây
    const boxes = [...(grp?.querySelectorAll('[data-testid^="class-facet-"]') ?? [])]; // chỉ hàng class (bỏ công tắc hybrid/confident và subclass)
    const label0 = boxes[0]?.closest('label')?.textContent ?? '';
    boxes[0]?.click();
    const chips = await wait(() => document.querySelector('[data-testid=filter-summary]'), 8000);
    const facetCount = Number((boxes[0]?.closest('label')?.querySelector('.fa-count')?.textContent ?? '').replace(/\D/g, ''));
    await wait(() => countNow() === facetCount, 10000);
    await sleep(300); await snap('class-filter');
    const total1 = countNow();
    await log('class-chip-and-filter', !!cchip && chipText.length > 0 && boxes.length > 0 && !!chips && total1 > 0 && total1 === facetCount && (boxes.length < 2 || total1 < total0), { facetCount, chipText, facets: boxes.length, label0, total0, total1, chips: chips?.textContent });
    await closeAdv(); await sleep(200);
    const sIn = document.querySelector('.fa-search input'); sIn?.focus();
    sIn?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Backspace', metaKey: true, bubbles: true, cancelable: true }));
    await wait(() => !document.querySelector('[data-testid=filter-summary]'), 5000);
    await wait(() => countNow() === total0, 8000); // số đếm làm mới bất đồng bộ sau khi bỏ nhóm lọc
    await log('class-filter-removed', countNow() === total0, { total0, now: countNow() });
    // Trùng lặp: mục sidebar khớp duplicate_summary thật; trang mới: thẻ hai bản cạnh nhau cùng dòng mẫu, câu thường, Giải quyết (Ẩn, không đụng tệp) → Đã giải quyết → Hoàn tác
    const dq = { text: null, vi: null, license: [], variable: null, italic: null, sourceIds: [], favoritesOnly: false, collectionId: null, tagIds: [], includeMissing: false, sort: 'family', descending: false, offset: 0, limit: 100, axisFilters: [], rawAxisRanges: [], hard: [], faceKeys: null, classes: [], subclasses: [], traits: [] };
    const waitA = async (fn, ms = 15000) => { const t = Date.now(); while (Date.now() - t < ms) { try { const v = await fn(); if (v) return v; } catch (e) {} await sleep(200); } return null; };
    const dsum = await inv('duplicate_summary', { q: dq });
    const dupBtn = () => [...document.querySelectorAll('[aria-label="Thanh bên"] button')].find(x => /Trùng lặp/.test(x.textContent));
    await wait(() => (dsum.total > 0) === !!dupBtn(), 8000);
    await log('duplicates-sidebar-item', (dsum.total > 0) === !!dupBtn() && (dsum.total === 0 || Number((dupBtn().textContent.match(/(\d+)\s*$/) || [])[1]) === dsum.total), { dsum, item: dupBtn()?.textContent ?? null });
    if (dsum.total > 0) {
      dupBtn().click();
      await wait(() => document.querySelector('.dup [data-testid=dup-group]'), 20000);
      await sleep(400); await snap('duplicates-view');
      const dplan = await inv('resolve_preview', { scope: { type: 'all' }, q: dq });
      const cards = [...document.querySelectorAll('[data-testid=dup-group]')];
      const dbtn = [...document.querySelectorAll('.dup button')].map(x => x.textContent.trim());
      const risky = dbtn.filter(x => /xoá|xóa|delete|di chuyển|move|gộp/i.test(x));
      const sides = cards.map(c => [...c.querySelectorAll('[data-testid=dup-spec]')].map(x => x.textContent));
      const sameSample = sides.every(x => x.length >= 2 && x.every(y => y === x[0] && /Việt Nam/.test(y)));
      const rawCodes = /same_content_tidiest|preferred_source|preferred_format|newer_version|newest_unknown|collection_file|may_reflow|chosen_by_user/.test(document.querySelector('.dup')?.textContent ?? '');
      const summary = document.querySelector('[data-testid=dup-summary]')?.textContent ?? '';
      const detailHidden = !document.querySelector('.dup [data-testid=dup-detail]');
      cards[0]?.querySelector('button[aria-expanded]')?.click();
      const detailShown = await wait(() => cards[0]?.querySelector('[data-testid=dup-detail]'), 3000);
      const openBtn = [...cards[0].querySelectorAll('button')].find(x => x.textContent.trim() === 'Mở chi tiết');
      openBtn?.click();
      const panel = await wait(() => document.querySelector('aside.fa-panel h2'), 15000);
      await log('duplicates-view', cards.length === dplan.items.length && cards.length === dsum.total && sameSample && !rawCodes && risky.length === 0 && detailHidden && !!detailShown && !!panel && summary.includes(`${dsum.total} nhóm`) && !/Lỗi giao diện/.test(document.body.textContent ?? ''), { cards: cards.length, plan: dplan.items.length, total: dsum.total, sides, sameSample, rawCodes, risky, detailHidden, shown: !!detailShown, panel: panel?.textContent, summary });
      // Giải quyết nhóm đầu bằng Ẩn (mặc định): không đụng tệp, nhóm biến mất, "Đã giải quyết" có dòng, Hoàn tác đưa nhóm về.
      const rev0 = await inv('resolve_list', { includeUndone: false });
      const firstCard = () => document.querySelector('.dup [data-testid=dup-group]');
      [...firstCard().querySelectorAll('button')].find(x => x.textContent.trim() === 'Giải quyết')?.click();
      const dlg = await wait(() => document.querySelector('[role=dialog] [data-testid=pv-summary]'), 10000);
      await sleep(300); await snap('duplicates-preview');
      const hideDefault = document.querySelector('[role=dialog] input[name=dup-act]')?.checked === true;
      document.querySelector('[data-testid=pv-confirm]')?.click();
      const gone = await waitA(async () => (await inv('duplicate_summary', { q: dq })).total === dsum.total - 1, 15000);
      await wait(() => !document.querySelector('[role=dialog]'), 8000);
      const rev1 = await inv('resolve_list', { includeUndone: false });
      await log('duplicates-resolve-hide', !!dlg && hideDefault && !!gone && rev1.length === rev0.length + 1 && rev1[0].action === 'hide', { hideDefault, before: rev0.length, after: rev1.length, action: rev1[0]?.action });
      [...document.querySelectorAll('.dup [role=tab]')].find(x => /Đã giải quyết/.test(x.textContent))?.click();
      const row = await wait(() => document.querySelector('[data-testid=dup-res-row]'), 10000);
      await sleep(300); await snap('duplicates-resolved');
      [...(row?.querySelectorAll('button') ?? [])].find(x => x.textContent.trim() === 'Hoàn tác')?.click();
      const back = await waitA(async () => (await inv('duplicate_summary', { q: dq })).total === dsum.total, 15000);
      await log('duplicates-undo', !!row && !!back && (await inv('resolve_list', { includeUndone: false })).length === rev0.length, { restored: !!back });
      [...document.querySelectorAll('.dup [role=tab]')].find(x => /Cần xử lý/.test(x.textContent))?.click();
    } else {
      await log('duplicates-view', true, 'không có nhóm trùng trong fixture QA: mục sidebar ẩn đúng');
    }
    // Thứ tự ưu tiên nguồn (Cài đặt → Thư viện): hàng khớp source_priority; nút ↓ đổi thứ tự thật rồi ↑ trả lại
    {
      const srcsNow = await inv('list_sources', {});
      if (srcsNow.length >= 2) {
        document.querySelector('button[aria-label="Cài đặt"]')?.click();
        await wait(() => document.querySelector('[data-testid=settings]'));
        [...document.querySelectorAll('.fa-set-nav button')].find(b => /Thư viện/.test(b.textContent))?.click();
        const box = await wait(() => document.querySelector('[data-testid=source-priority]'), 8000);
        const order0 = await inv('source_priority', {});
        const rowsN = () => [...document.querySelectorAll('[data-testid=prio-row] strong')].map(x => x.textContent);
        const nameOf = id => srcsNow.find(s => s.id === id)?.name;
        const match0 = JSON.stringify(rowsN()) === JSON.stringify(order0.map(nameOf));
        await sleep(200); await snap('source-priority');
        document.querySelector('[data-testid=prio-row] .mv button:nth-child(2)')?.click();
        const moved = await waitA(async () => { const o = await inv('source_priority', {}); return o[0] === order0[1] && o[1] === order0[0]; }, 8000);
        document.querySelector('[data-testid=prio-row]:nth-child(2) .mv button:nth-child(1)')?.click();
        const restored = await waitA(async () => JSON.stringify(await inv('source_priority', {})) === JSON.stringify(order0), 8000);
        await log('source-priority', !!box && match0 && !!moved && !!restored, { rows: rowsN(), order0, match0, moved: !!moved, restored: !!restored });
        [...document.querySelectorAll('[aria-label="Thanh bên"] button')].find(b => b.textContent.includes('Tất cả'))?.click();
        await wait(() => !document.querySelector('[data-testid=settings]') && cells().length > 0, 10000);
      } else {
        await log('source-priority', true, 'ít hơn 2 nguồn trong fixture QA: khối thứ tự ưu tiên ẩn đúng');
      }
    }
    // Nhóm theo Family (R3): ⌥G bật → tiêu đề nhóm khớp family_groups, số hàng khớp, ←/→ thu gọn/mở, ⌥G tắt lại
    {
      [...document.querySelectorAll('[aria-label="Thanh bên"] button')].find(b => b.textContent.includes('Tất cả'))?.click();
      await wait(() => cells().length > 0, 10000);
      document.activeElement?.blur?.();
      const gq = { ...dq, groupByFamily: true, limit: 100 };
      const fg = await inv('family_groups', { q: gq });
      const totalQ = (await inv('query_faces', { q: { ...dq, limit: 1 } })).total;
      const sumCount = fg.reduce((a, g) => a + g.count, 0);
      const altG = () => document.dispatchEvent(new KeyboardEvent('keydown', { key: 'g', code: 'KeyG', altKey: true, bubbles: true, cancelable: true }));
      altG();
      const head0 = await waitA(() => document.querySelector('[data-testid=group-head]'), 15000);
      await sleep(500); await snap('family-groups');
      const grid = document.querySelector('[role=grid]');
      const colsN = Number(grid?.getAttribute('aria-colcount')) || 1;
      const expectRows = fg.length + fg.reduce((a, g) => a + Math.ceil(g.count / colsN), 0);
      const rowsNow = Number(grid?.getAttribute('aria-rowcount'));
      const label0 = document.querySelector('[data-testid=group-count]')?.textContent ?? '';
      const g0 = fg[0];
      const wantLabel = g0 && g0.matchCount < g0.styleTotal ? `${g0.matchCount}/${g0.styleTotal} kiểu` : `${g0?.styleTotal} kiểu`;
      const topOf = el => el?.getBoundingClientRect().top;
      const headsNow = () => [...document.querySelectorAll('[data-testid=group-head]')];
      const gap0 = headsNow().length >= 2 ? topOf(headsNow()[1]) - topOf(headsNow()[0]) : null;
      headsNow()[0]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true, cancelable: true }));
      await waitA(() => headsNow()[0]?.getAttribute('aria-expanded') === 'false', 5000);
      await sleep(300);
      const gap1 = headsNow().length >= 2 ? topOf(headsNow()[1]) - topOf(headsNow()[0]) : null;
      const collapsedOk = headsNow()[0]?.getAttribute('aria-expanded') === 'false' && (gap0 === null || gap1 === null || (gap1 < gap0 && Math.abs(gap1 - 32) <= 1));
      headsNow()[0]?.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true, cancelable: true }));
      const reopened = await waitA(() => headsNow()[0]?.getAttribute('aria-expanded') === 'true', 5000);
      altG();
      const off = await waitA(() => !document.querySelector('[data-testid=group-head]') && !document.querySelector('[role=grid]')?.dataset.grouped, 8000);
      await log('family-groups', !!head0 && sumCount === totalQ && fg.length > 0 && rowsNow === expectRows && label0 === wantLabel && collapsedOk && !!reopened && !!off, { groups: fg.length, sumCount, totalQ, colsN, expectRows, rowsNow, label0, wantLabel, gap0, gap1, collapsedOk, reopened: !!reopened, off: !!off });
    }
    // Gỡ nguồn: menu chuột phải → hộp xem trước có số liệu khớp remove_source_preview; Huỷ không gỡ gì
    {
      const srcs2 = await inv('list_sources', {});
      const target = srcs2.find(x => x.available) ?? srcs2[0];
      // đúng mục nguồn: theo title (đường dẫn) hoặc tên đứng đầu rồi tới số đếm — KHÔNG dùng includes (tên nguồn có thể chỉ một chữ cái)
      const srcBtns = [...document.querySelectorAll('[aria-label="Thanh bên"] button')];
      const item = target && (srcBtns.find(b => target.path && b.getAttribute('title') === target.path) ?? srcBtns.find(b => b.textContent.startsWith(target.name) && /^\d*$/.test(b.textContent.slice(target.name.length).replace(/\D.*$/, ''))));
      if (item) {
        const prev = await inv('remove_source_preview', { sourceId: target.id });
        const r = item.getBoundingClientRect();
        item.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r.left + 10, clientY: r.top + 5 }));
        const mi = await waitA(() => [...document.querySelectorAll('[role=menuitem]')].find(x => /Gỡ nguồn/.test(x.textContent)), 5000);
        mi?.click();
        const facts = await waitA(() => document.querySelector('[data-testid=rmsrc-facts]'), 8000);
        await sleep(300); await snap('remove-source-preview');
        const locText = facts?.querySelector('[data-fact=locations]')?.textContent ?? '';
        const locOk = locText.includes(`${prev.removedLocations} vị trí`);
        const noDelete = /không xoá file font nào/.test(document.querySelector('[data-testid=rmsrc]')?.textContent ?? '');
        document.querySelector('[data-testid=rmsrc-cancel]')?.click();
        document.querySelector('[role=menu]')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true })); // không để menu mở chặn ⌘A của bước sau
        const closed = await waitA(() => !document.querySelector('[data-testid=rmsrc]'), 5000);
        const after = await inv('list_sources', {});
        await log('remove-source-preview', !!facts && locOk && noDelete && !!closed && after.length === srcs2.length && after.some(x => x.id === target.id), { target: target.name, prev, locText, locOk, noDelete, closed: !!closed, before: srcs2.length, after: after.length });
      } else {
        await log('remove-source-preview', true, 'không có nguồn trong fixture QA');
      }
    }
    // XUNG ĐỘT KÍCH HOẠT (2c-A) + TAG NỔI (2c-B). Thêm nguồn E: 12 bản sửa 2 byte của font đã có (cùng tên PostScript, khác nội dung).
    // Chọn tất cả → Activate: một nửa mỗi cặp xung đột thật (engine CoreText thật, chỉ font fixture trong thư mục QA).
    {
      const actOn = async () => (await inv('activation_state', {})).filter(x => x.state === 'active');
      const countNow2 = () => Number((document.querySelector('[data-testid=view-count]')?.textContent ?? '').replace(/\D/g, '')) || 0;
      [...document.querySelectorAll('[aria-label="Thanh bên"] button')].find(b => b.textContent.includes('Tất cả'))?.click();
      await sleep(600);
      const base = countNow2();
      await inv('add_source', { path: '__QA_DIR__/E' });
      await wait(() => countNow2() >= base + 12, 120000);
      await sleep(800);
      const total = countNow2();
      // Hộp thoại Gỡ nguồn (bước trước) trả focus về nút nguồn ở thanh bên: ⌘A chỉ chạy khi focus ở thư viện hoặc ngoài mọi vùng, nên bỏ focus như người dùng bấm ra vùng trống.
      (document.activeElement instanceof HTMLElement) && document.activeElement.blur();
      const gridTop = () => document.querySelector('[role=grid]')?.getBoundingClientRect().top;
      // chọn 3 font (cmd-click) để đo vị trí lưới; sau đó chọn tất cả: tag nổi hiện mà lưới KHÔNG đổi chỗ.
      const c3 = cells(); c3[0].click(); c3[1].dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, metaKey: true })); c3[2].dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, metaKey: true }));
      await wait(() => /^3 đã chọn$/.test(document.querySelector('[data-testid=selection-count]')?.textContent ?? ''), 8000);
      const top3 = gridTop(); const noTag3 = !document.querySelector('[data-testid=narrow-tag]');
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'a', metaKey: true, bubbles: true, cancelable: true }));
      await wait(() => Number((document.querySelector('[data-testid=selection-count]')?.textContent ?? '').replace(/\D/g, '')) >= total, 15000);
      await sleep(300);
      const topAll = gridTop(); const tag = document.querySelector('[data-testid=narrow-tag]');
      const tagBox = tag?.getBoundingClientRect(); const wrapBox = tag?.closest('.fa-act-wrap')?.getBoundingClientRect(); const gb = document.querySelector('[role=grid]')?.getBoundingClientRect();
      await snap('narrow-tag');
      await log('narrow-tag-no-layout-shift', noTag3 && !!tag && /^\d+ · nên ≤ 4$/.test(tag.textContent.replace('!', '')) && Math.abs(topAll - top3) <= 1 && !!wrapBox && !document.querySelector('.fa-act-hint'), { top3, topAll, noTag3, total, sel: document.querySelector('[data-testid=selection-count]')?.textContent, blockers: [...document.querySelectorAll('[role=menu],[role=dialog],[role=alertdialog]')].map(e => e.getAttribute('role') + ':' + (e.getAttribute('aria-label') ?? e.className)), active: document.activeElement?.tagName + '.' + document.activeElement?.className, text: tag?.textContent, tagBottom: tagBox && Math.round(tagBox.bottom), gridTop: gb && Math.round(gb.top) });
      tag?.dispatchEvent(new PointerEvent('pointerover', { bubbles: true, pointerType: 'mouse' })); tag?.focus();
      const pop = await wait(() => document.querySelector('[data-testid=narrow-pop]'), 4000);
      await log('narrow-pop-has-action', !!pop && !!btn('Chỉ lấy 4 đầu', pop) && Math.abs(gridTop() - top3) <= 1, { pop: pop?.textContent });
      tag?.blur?.(); document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
      // Chọn tất cả rồi Activate.
      await wait(() => !document.querySelector('[data-testid=narrow-pop]'), 3000);
      const selBtn = document.querySelector('[data-testid=selbar-activate]'); selBtn?.focus(); selBtn?.click();
      const dlg = await wait(() => document.querySelector('[role=dialog].fa-conf'), 30000);
      await sleep(300);
      const rowsN = dlg ? dlg.querySelectorAll('[data-testid=conflict-list] > li').length : 0;
      const h = dlg?.getBoundingClientRect().height ?? 0;
      // Hộp thoại phải NẰM TRÊN CÙNG thật (không kẹt dưới stacking context của Top Bar/lưới): điểm giữa và 4 góc trong hộp trỏ vào chính nó.
      const dr = dlg?.getBoundingClientRect();
      const probes = dr ? [[0.5, 0.5], [0.1, 0.1], [0.9, 0.1], [0.1, 0.9], [0.9, 0.9]].map(([fx, fy]) => dlg.contains(document.elementFromPoint(dr.left + dr.width * fx, dr.top + dr.height * fy))) : [];
      const scrimTop = !!dlg && document.elementFromPoint(20, window.innerHeight - 20)?.classList.contains('fa-modal-scrim');
      await log('conflict-dialog-on-top', probes.length === 5 && probes.every(Boolean) && scrimTop && dlg.parentElement?.parentElement === document.body, { probes, scrimTop, parent: dlg?.parentElement?.parentElement?.tagName });
      await sleep(400); await snap('conflict-dialog');
      const themeC = document.documentElement.dataset.theme; document.documentElement.dataset.theme = 'dark'; await sleep(400); await snap('conflict-dialog-dark');
      if (themeC) document.documentElement.dataset.theme = themeC; else document.documentElement.removeAttribute('data-theme'); await sleep(200);
      const title = dlg?.querySelector('h2')?.textContent ?? '';
      await log('conflict-dialog-compact', !!dlg && rowsN >= 6 && h <= window.innerHeight * 0.7 + 1 && /^\d+ font đã có trên máy$/.test(title) && !/\/Users\/|\/var\//.test(dlg.textContent ?? '') && !!dlg.querySelector('[role=img][aria-label*="· /"]'), { rowsN, h: Math.round(h), win: window.innerHeight, title, text: (dlg?.textContent ?? '').slice(0, 200) });
      const radios = [...(dlg?.querySelectorAll('input[type=radio]') ?? [])];
      const rs = radios.map(r => { const cs = getComputedStyle(r), af = getComputedStyle(r, '::after'); return { appearance: cs.appearance, w: cs.width, bw: cs.borderTopWidth, br: cs.borderTopLeftRadius, dot: af.width, checked: r.checked }; });
      await log('conflict-radio-ring-and-dot', (rs.length === 2 || rs.length === 3) && rs.every(x => x.appearance === 'none' && x.w === '16px' && parseFloat(x.bw) >= 1 && x.dot === '8px'), { rs });
      const exp = dlg?.querySelector('button[aria-expanded=false]'); exp?.click();
      const styles = await wait(() => dlg?.querySelector('.fa-conf-styles'), 5000);
      await log('conflict-dialog-expand', !!exp && !!styles && exp.getAttribute('aria-expanded') === 'true', { expanded: exp?.getAttribute('aria-expanded') });
      const goBtn = btn('Tiếp tục', dlg); goBtn?.click();
      await wait(() => !document.querySelector('[role=dialog].fa-conf'), 30000);
      await log('conflict-dialog-returns-focus', document.activeElement === selBtn || document.activeElement?.closest('[data-testid=selbar-activate]') === selBtn, { active: document.activeElement?.tagName + '.' + (document.activeElement?.getAttribute('data-testid') ?? '') });
      await sleep(800);
      let st2 = await actOn();
      await log('conflict-skip-activates-rest', st2.length >= 12 && st2.length <= total - 6, { active: st2.length, total });
      // Dọn: tắt mọi font đã bật (command thật, không động tới thứ gì ngoài các font QA vừa bật).
      try { await inv('activation_deactivate_all', { onProgress: '__CHANNEL__:' + T.transformCallback(() => {}) }); } catch (e) { await log('conflict-clean-error', false, String(e && e.message || e)); }
      await sleep(800); st2 = await actOn();
      await log('conflict-clean', st2.length === 0, { left: st2.length });
    }

    // "CÓ SẴN TRÊN MÁY": một bản font hệ thống chép vào nguồn G (giống hệt từng byte) → Font Atlas không bật gì, chỉ báo ai cung cấp.
    // (Chỉ là phép thử trong thư mục QA tạm; không ảnh nào của bước này được đưa vào tài liệu.)
    {
      const actOn = async () => (await inv('activation_state', {})).filter(x => x.state === 'active');
      const dqa = { text: 'Academy', vi: null, license: [], variable: null, italic: null, sourceIds: [], favoritesOnly: false, collectionId: null, tagIds: [], includeMissing: false, sort: 'family', descending: false, offset: 0, limit: 10, axisFilters: [], rawAxisRanges: [], hard: [], faceKeys: null, classes: [], subclasses: [], traits: [] };
      await inv('add_source', { path: '__QA_DIR__/G' });
      let gf = null; const t0 = Date.now();
      while (!gf && Date.now() - t0 < 60000) { const p = await inv('query_faces', { q: dqa }); gf = (p.items || [])[0] || null; if (!gf) await sleep(500); }
      if (!gf) { await log('available-outcome', true, 'máy không có font hệ thống mẫu: bỏ qua'); }
      else {
        const key = { hashHex: gf.hashHex ?? gf.hash_hex, faceIndex: gf.faceIndex ?? gf.face_index ?? 0 };
        const rep = await inv('activation_activate', { faces: [key], scope: null, proceed: false, onProgress: '__CHANNEL__:' + T.transformCallback(() => {}) });
        const o = rep.results[0].outcome; const stA = await actOn();
        await log('available-outcome', o.kind === 'already_available' && o.owner === 'system' && stA.length === 0 && rep.changed === 0, { outcome: o.kind, owner: o.owner, active: stA.length });
        const again = await inv('activation_availability', { faces: [] });
        await log('available-recomputed', again.length === 1 && again[0].hashHex === key.hashHex, { n: again.length });
        window.dispatchEvent(new Event('focus'));
        [...document.querySelectorAll('[aria-label="Thanh bên"] button')].find(b => /^G\d*$/.test(b.textContent.trim()))?.click();
        await sleep(800);
        const cell = await wait(() => cells()[0], 10000); cell?.click();
        const rowA = await wait(() => document.querySelector('aside.fa-panel [data-testid=active-row][data-available=true]'), 10000);
        const swA = rowA?.querySelector('[data-testid=active-switch]');
        const dotA = cells()[0]?.querySelector('[data-testid=active-dot][data-scope=available]');
        await sleep(300); await snap('available-peek');
        await log('available-peek-and-card', !!rowA && swA.getAttribute('aria-disabled') === 'true' && /^Có sẵn · /.test(rowA.querySelector('[data-testid=active-status]').textContent) && !!dotA && !cells()[0].querySelector('[data-testid=cell-active]') && (await actOn()).length === 0, { status: rowA?.querySelector('[data-testid=active-status]')?.textContent, dot: !!dotA, ncells: cells().length, cell0: cells()[0]?.textContent?.slice(0, 40), anyRow: !!document.querySelector('aside.fa-panel [data-testid=active-row]'), rowStatus: document.querySelector('aside.fa-panel [data-testid=active-status]')?.textContent, rowAvailAttr: document.querySelector('aside.fa-panel [data-testid=active-row]')?.getAttribute('data-available'), dotAny: cells()[0]?.querySelector('[data-testid=active-dot]')?.getAttribute('data-scope'), side: [...document.querySelectorAll('[aria-label="Thanh bên"] button')].map(b => b.textContent.trim()).slice(-8) });
      }
    }
    // PAIRING DESK (2b lát 1–3): Pair từ peek → Desk; ↓ thử, Enter đặt; dải chi tiết có Δ; Combo stepper; Esc về Thư viện
    {
      const deskEl = () => document.querySelector('[data-testid=pairing-desk]');
      const pressOn = (el, key, extra = {}) => el.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...extra }));
      [...document.querySelectorAll('[aria-label="Thanh bên"] button')].find(b => /^Tất cả/.test(b.textContent.trim()))?.click();
      await sleep(800);
      const cell0 = await wait(() => cells().find(c => /Playfair Display/.test(c.textContent)), 10000);
      cell0?.click(); await sleep(400);
      const pairTab = await wait(() => document.querySelector('aside.fa-panel [role=tab][aria-label="Ghép cặp"]'), 8000);
      pairTab?.click();
      const openBtn = await wait(() => document.querySelector('[data-testid=open-desk]'), 15000);
      openBtn?.click();
      const desk = await wait(() => deskEl(), 15000);
      await wait(() => document.querySelectorAll('[data-testid=desk-cand]').length > 0 || document.querySelector('[data-testid=desk-noanchor]'), 15000);
      await sleep(500);
      await log('desk-opens', !!desk && document.querySelectorAll('[data-testid=desk-role]').length >= 2 && !!document.querySelector('[data-testid=desk-heading]'), { roles: document.querySelectorAll('[data-testid=desk-role]').length, cands: document.querySelectorAll('[data-testid=desk-cand]').length });
      const tb = document.querySelector('header.fa-topbar');
      await log('desk-is-the-only-page', !!desk && (!tb || tb.offsetParent === null) && !document.querySelector('.cmp-tbl, .pr') && !document.querySelector('[data-testid=duplicates-page]')?.offsetParent, { topbarHidden: !tb || tb.offsetParent === null });
      if (desk) {
        document.documentElement.dataset.theme = 'light'; await sleep(250); await snap('font-desk-light');
        pressOn(desk, 'Enter');
        const nFilled = () => [...document.querySelectorAll('[data-testid=desk-role]')].filter(r => !/Chưa có font/.test(r.textContent)).length;
        await wait(() => nFilled() >= 2, 6000);
        const filled = nFilled();
        await log('desk-enter-places', filled >= 2, { filled });
        const nCand = document.querySelectorAll('[data-testid=desk-cand]').length;
        pressOn(desk, 'ArrowDown'); await sleep(900);
        const deltas = [...document.querySelectorAll('[data-testid=strip-delta]')].map(d => d.textContent);
        await log('desk-strip-delta', deltas.length === 8 && (nCand < 2 || deltas.some(x => /^[+−]|±0/.test(x))), { nCand, deltas });
        pressOn(desk, 'i'); await sleep(150);
        const collapsed = document.querySelector('[data-testid=detail-strip]')?.dataset.collapsed === 'true'; pressOn(desk, 'i'); await sleep(150);
        await log('desk-strip-collapses', collapsed && !!document.querySelector('[data-testid=strip-metrics]'), { collapsed });
        document.querySelector('[data-testid=combo-label]')?.click(); await sleep(200);
        [...document.querySelectorAll('[role=menuitem]')].find(i => /Combo mới/.test(i.textContent))?.click(); await sleep(700);
        const l2 = document.querySelector('[data-testid=combo-label]')?.textContent ?? '';
        pressOn(deskEl(), '['); await sleep(500);
        const l1 = document.querySelector('[data-testid=combo-label]')?.textContent ?? '';
        await log('desk-combo-stepper', /Combo 2\/2/.test(l2) && /Combo 1\/2/.test(l1), { l2, l1 });
        document.querySelector('[data-testid=mode-combos]')?.click();
        await wait(() => document.querySelectorAll('[data-testid=cc-col]').length >= 2, 10000);
        await sleep(400);
        const ccCols = document.querySelectorAll('[data-testid=cc-col]').length;
        await log('desk-compare-combos', ccCols === 2 && document.querySelector('[data-testid=cc-col]')?.dataset.base === 'true' && document.querySelectorAll('[data-testid=cc-rel]').length === 2, { ccCols });
        for (const th of ['dark', 'light']) { document.documentElement.dataset.theme = th; await sleep(250); await snap('font-desk-compare-combos-' + th); }
        document.querySelector('[data-testid=mode-fonts]')?.click();
        await wait(() => !!document.querySelector('[data-testid=font-compare]'), 8000);
        await sleep(500);
        await log('desk-compare-fonts', !!document.querySelector('[data-testid=font-compare]') && document.querySelectorAll('tr[data-metric]').length >= 4 && !!document.querySelector('[data-testid=fc-sort]'), { cols: document.querySelectorAll('[data-testid=fc-col]').length });
        for (const th of ['dark', 'light']) { document.documentElement.dataset.theme = th; await sleep(250); await snap('font-desk-compare-fonts-' + th); }
        document.querySelector('[data-testid=fc-view-pages]')?.click(); await sleep(400);
        await log('desk-compare-fonts-pages', !document.querySelector('tr[data-metric]'), {});
        // Đặt vào vai từ Compare Font: ứng viên đầu thành MỐC (cột đầu), không còn lặp ở cột khác.
        {
          // Cần ít nhất một cột ứng viên: chờ gợi ý (≤5s); không có thì thêm từ thư viện bằng ô "Thêm cột" (font fixture khác mốc).
          await wait(() => document.querySelectorAll('[data-testid=fc-col]').length >= 2, 5000);
          if (document.querySelectorAll('[data-testid=fc-col]').length < 2) {
            const sel = document.querySelector('[data-testid=fc-add]');
            await wait(() => sel && sel.options.length > 1 && !sel.disabled, 8000);
            if (sel && sel.options.length > 1) { sel.value = sel.options[1].value; sel.dispatchEvent(new Event('change', { bubbles: true })); }
            await wait(() => document.querySelectorAll('[data-testid=fc-col]').length >= 2, 5000);
          }
          const placeBtn = document.querySelector('[data-testid=fc-place]');
          const pname = placeBtn?.closest('th')?.querySelector('.fa-cc-title')?.textContent?.trim() ?? '';
          {
          placeBtn?.click();
          await wait(() => { const f = document.querySelector('[data-testid=fc-col]'); return f?.dataset.base === 'true' && (f.querySelector('.fa-cc-title')?.textContent ?? '').includes(pname); }, 8000);
          const titles = [...document.querySelectorAll('[data-testid=fc-col] .fa-cc-title')].map(x => x.textContent.replace(/\s*·\s*mốc$/, '').trim());
          await log('desk-compare-fonts-place', !!placeBtn && pname.length > 0 && titles[0] === pname && new Set(titles).size === titles.length, { pname, titles: titles.length, first: titles[0] });
          }
        }
        document.querySelector('[data-testid=mode-pairing]')?.click(); await sleep(400);
        for (const th of ['dark', 'light']) { document.documentElement.dataset.theme = th; await sleep(250); await snap('font-desk-' + th); }
        pressOn(deskEl(), 'Escape');
        await wait(() => !deskEl(), 8000);
        await log('desk-esc-leaves', !deskEl() && !!cells().length > 0, {});
      }
    }

    // Hiển thị (A3): mở bằng nút, bật Giấy phép → thẻ có hàng 2, Đặt lại, Esc trả focus về nút.
    {
      const vb = () => document.querySelector('[data-testid=view-open]');
      if (vb()) {
        vb().click();
        await wait(() => !!document.querySelector('[data-testid=view-pop]'), 5000);
        const cellH = () => document.querySelector('[role=gridcell][data-face-id]')?.getBoundingClientRect().height ?? 0;
        const h0 = cellH();
        const before = document.querySelectorAll('[data-testid=face-meta2]').length;
        document.querySelector('[data-field=license] input')?.click();
        await wait(() => document.querySelectorAll('[data-testid=face-license]').length > 0, 8000);
        const after = document.querySelectorAll('[data-testid=face-meta2]').length;
        await sleep(300);
        const h1 = cellH();
        for (const th of ['dark', 'light']) { document.documentElement.dataset.theme = th; await sleep(250); await snap('view-options-' + th); }
        document.querySelector('[data-testid=view-reset]')?.click();
        await wait(() => document.querySelectorAll('[data-testid=face-license]').length === 0, 8000);
        (document.activeElement && document.querySelector('[data-testid=view-pop]')?.contains(document.activeElement) ? document.activeElement : document.querySelector('[data-testid=view-pop]') ?? document.body).dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
        await sleep(300);
        await log('view-options', before === 0 && after > 0 && !document.querySelector('[data-testid=view-pop]'), { before, after, h0, h1, popOpen: !!document.querySelector('[data-testid=view-pop]') });
        await wait(() => document.activeElement === vb(), 1000);
        await log('view-options-focus-returns', document.activeElement === vb(), { active: document.activeElement?.tagName + '.' + (document.activeElement?.getAttribute('data-testid') ?? '') });
        await log('view-options-row-height', h0 > 0 && Math.abs((h1 - h0) - 18) <= 2, { h0, h1 });
      } else await log('view-options', false, 'no view-open button');
    }

    // Menu chuột phải Sidebar (C6): mở trên "Gần đây" không đổi view; Esc đóng và trả focus.
    {
      const nav = [...document.querySelectorAll('[data-testid=sidebar] .fa-nav')].find(b => /Gần đây/.test(b.textContent));
      if (nav) {
        const viewBefore = document.querySelector('[data-testid=sidebar] .fa-nav[aria-current=true]')?.textContent;
        const r = nav.getBoundingClientRect();
        nav.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: r.left + 20, clientY: r.top + 10 }));
        await wait(() => !!document.querySelector('[role=menu]'), 5000);
        await wait(() => nav.dataset.context === 'true', 3000);
        const opened = !!document.querySelector('[role=menu]') && nav.dataset.context === 'true';
        const menuLabel = document.querySelector('[role=menu]')?.getAttribute('aria-label') ?? null;
        await snap('sidebar-context-menu');
        (document.querySelector('[role=menu]') ?? document.body).dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
        // Đóng menu và trả focus xảy ra bất đồng bộ (máy tải nặng có thể trễ): chờ điều kiện, không đọc ngay.
        const closed = !!(await wait(() => !document.querySelector('[role=menu]'), 3000));
        const focused = !!(await wait(() => document.activeElement === nav, 3000));
        const sameView = !!(await wait(() => document.querySelector('[data-testid=sidebar] .fa-nav[aria-current=true]')?.textContent === viewBefore, 3000));
        const viewAfter = document.querySelector('[data-testid=sidebar] .fa-nav[aria-current=true]')?.textContent;
        await log('sidebar-context-menu', opened && closed && sameView && focused, { opened, closed, focused, sameView, menuLabel, viewBefore, viewAfter, context: nav.dataset.context ?? null, active: document.activeElement?.tagName + '.' + (document.activeElement?.className ?? '') });
      } else await log('sidebar-context-menu', false, 'no Gần đây item');
    }

    await log('done', true, '');
  } catch (e) { await log('error', false, String(e && e.stack || e)); }
})();
