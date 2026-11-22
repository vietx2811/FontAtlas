// FontAtlas — phần chạy trong Illustrator (ExtendScript), gọi từ panel nền qua evalScript.
// Không sửa tài liệu, không mở hộp thoại, và KHÔNG giữ luồng chính của Illustrator lâu: quét theo từng lát thời gian.
//
// faSig(): chữ ký RẺ của tập tài liệu đang mở ("tên|đường dẫn|số text frame" mỗi dòng): panel chỉ quét khi nó đổi.
// faScan(reset, budgetMs): quét font từng text run (mỗi run có thể một font riêng, không chỉ font đầu của frame), giữ lại
//   font KHÔNG có trong app.textFonts. Chạy tối đa budgetMs rồi trả {done:false}; gọi lại (reset=false) để tiếp tục từ chỗ cũ.
//   Xong thì trả {done:true, docs:[{name,path,fonts:[{ps,family,style}]}], stats:{...}}. `stats` chỉ để chẩn đoán (panel ghi vào tệp log):
//   số tài liệu/frame/run đã quét, số run không đọc được font, tối đa 12 tên font đã thấy (kèm cờ thiếu) và tối đa 3 thông điệp lỗi
//   (đã cắt ngắn, KHÔNG có tên tài liệu). Lỗi đọc font KHÔNG còn bị nuốt im lặng.
// ExtendScript (ES3) KHÔNG có JSON: tự dựng chuỗi JSON.
function faQ(v) {
  return '"' + String(v).replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/[\u0000-\u001f\u2028\u2029]/g, " ") + '"';
}
function faDocsJson(docs) {
  var out = [], i, j, d, fs;
  for (i = 0; i < docs.length; i++) {
    d = docs[i]; fs = [];
    if (!d) continue;
    for (j = 0; j < d.fonts.length; j++) fs.push('{"ps":' + faQ(d.fonts[j].ps) + ',"family":' + faQ(d.fonts[j].family) + ',"style":' + faQ(d.fonts[j].style) + '}');
    out.push('{"name":' + faQ(d.name) + ',"path":' + faQ(d.path) + ',"fonts":[' + fs.join(",") + ']}');
  }
  return "[" + out.join(",") + "]";
}

// ---- chẩn đoán (không ảnh hưởng kết quả quét) ----
function faNote(st, kind, e) {
  var m = kind + ":" + String(e && (e.message || e)).replace(/[\u0000-\u001f]/g, " ").substring(0, 80), i;
  st.stats.errs++;
  for (i = 0; i < st.errMsgs.length; i++) if (st.errMsgs[i] === m) return;
  if (st.errMsgs.length < 3) st.errMsgs.push(m);
}
function faSeen(st, ps, miss) {
  var i;
  for (i = 0; i < st.seenList.length; i++) if (st.seenList[i].ps === ps) return;
  if (st.seenList.length < 12) st.seenList.push({ ps: ps, miss: miss ? 1 : 0 });
}
function faStatsJson(st) {
  var a = [], b = [], i, s = st.stats;
  for (i = 0; i < st.seenList.length; i++) a.push('{"ps":' + faQ(st.seenList[i].ps) + ',"miss":' + st.seenList[i].miss + "}");
  for (i = 0; i < st.errMsgs.length; i++) b.push(faQ(st.errMsgs[i]));
  return '{"docs":' + s.docs + ',"frames":' + s.frames + ',"runs":' + s.runs + ',"noFont":' + s.noFont + ',"errs":' + s.errs +
    ',"installed":' + s.installed + ',"viaRuns":' + (s.via_runs || 0) + ',"viaParas":' + (s.via_paras || 0) + ',"viaFrame":' + (s.via_frame || 0) + ',"probe":' + faQ(st.probe || "") + ',"seen":[' + a.join(",") + '],"errMsgs":[' + b.join(",") + "]}";
}

function faSig() {
  try {
    var a = [], i, d, p;
    for (i = 0; i < app.documents.length; i++) {
      d = app.documents[i]; p = "";
      try { p = d.fullName.fsName; } catch (e) {}
      a.push(d.name + "|" + p + "|" + d.textFrames.length);
    }
    return a.join("\n");
  } catch (e) { return "err"; }
}

function faScan(reset, budgetMs) {
  var g = $.global, st = g.__fa, t0 = new Date().getTime(), i, r;
  try {
    if (reset || !st) st = g.__fa = { installed: null, di: 0, fi: 0, ri: 0, docs: [], seenList: [], errMsgs: [], stats: { docs: 0, frames: 0, runs: 0, noFont: 0, errs: 0, installed: 0 } };
    if (!st.installed) {
      st.installed = {};
      var all = app.textFonts;
      for (i = 0; i < all.length; i++) { try { st.installed[all[i].name] = 1; st.stats.installed++; } catch (e) { faNote(st, "textFonts", e); } }
    }
    while (st.di < app.documents.length) {
      var d = app.documents[st.di];
      if (!st.docs[st.di]) {
        var path = ""; try { path = d.fullName.fsName; } catch (e) {}
        st.docs[st.di] = { name: String(d.name), path: path, fonts: [], seen: {} };
        st.stats.docs++;
      }
      var rec = st.docs[st.di], frames = [];
      try { frames = d.textFrames; } catch (e) { faNote(st, "frames", e); }
      while (st.fi < frames.length) {
        // Vị trí quét gồm cả chỉ số text run (st.ri): frame có rất nhiều run được quét tiếp ở lát sau, không cắt bớt.
        // Nguồn các đoạn chữ để đọc font: (1) textRuns; nếu rỗng/length không phải số (Illustrator 2027 thấy 0 run dù có chữ) thì
        // (2) từng paragraph; nếu vẫn không có thì (3) cả textRange của frame. Chọn lại (idempotent) mỗi lần vào frame, kể cả khi quét tiếp ở lát sau.
        var items = null, n = 0, kind = "runs", fr = frames[st.fi], rg = null;
        try { rg = fr.textRange; } catch (e) { faNote(st, "range", e); }
        try { items = rg ? rg.textRuns : null; n = items ? Number(items.length) : 0; if (isNaN(n)) n = 0; } catch (e) { faNote(st, "runs", e); items = null; n = 0; }
        var n0 = n, pn = -1;
        if (!(n > 0) && rg) {
          kind = "paras";
          try { items = rg.paragraphs; n = items ? Number(items.length) : 0; if (isNaN(n)) n = 0; } catch (e) { faNote(st, "paras", e); items = null; n = 0; }
          pn = n;
          if (!(n > 0)) { kind = "frame"; items = [rg]; n = 1; }
        }
        if (!st.probe) st.probe = "runs=" + n0 + " paras=" + pn + " via=" + kind;
        while (st.ri < n) {
          if (new Date().getTime() - t0 > budgetMs) return '{"ok":true,"done":false}';
          var f = null, ps = "", fam = "", sty = "";
          try { f = items[st.ri].characterAttributes.textFont; } catch (e) { faNote(st, kind === "runs" ? "textFont" : kind, e); }
          if (f) {
            try { ps = f.name; } catch (e) { faNote(st, "name", e); }
            try { fam = f.family || ""; } catch (e) {}
            try { sty = f.style || ""; } catch (e) {}
          } else st.stats.noFont++;
          st.stats.runs++;
          st.stats["via_" + kind] = (st.stats["via_" + kind] || 0) + 1;
          if (ps) {
            var miss = !st.installed[ps];
            faSeen(st, String(ps), miss);
            if (miss && !rec.seen[ps]) {
              rec.seen[ps] = 1;
              rec.fonts.push({ ps: String(ps), family: String(fam), style: String(sty) });
            }
          }
          st.ri++;
        }
        st.fi++; st.ri = 0; st.stats.frames++;
        if (new Date().getTime() - t0 > budgetMs) return '{"ok":true,"done":false}';
      }
      st.di++; st.fi = 0; st.ri = 0;
    }
    var docs = st.docs, stats = faStatsJson(st);
    g.__fa = null;
    return '{"ok":true,"done":true,"docs":' + faDocsJson(docs) + ',"stats":' + stats + "}";
  } catch (e) {
    g.__fa = null;
    return '{"ok":false,"error":' + faQ(String(e && (e.message || e)).substring(0, 120)) + "}";
  }
}

function faRedraw() { try { app.redraw(); return "ok"; } catch (e) { return "err:" + e; } }
