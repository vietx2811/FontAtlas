// FontAtlas — phần chạy trong Illustrator (ExtendScript), gọi từ panel nền qua evalScript.
// Không sửa tài liệu, không mở hộp thoại, và KHÔNG giữ luồng chính của Illustrator lâu: quét theo từng lát thời gian.
//
// faSig(): chữ ký RẺ của tập tài liệu đang mở ("tên|đường dẫn|số text frame" mỗi dòng): panel chỉ quét khi nó đổi.
// faScan(reset, budgetMs): quét font từng text run (mỗi run có thể một font riêng, không chỉ font đầu của frame), giữ lại
//   font KHÔNG có trong app.textFonts. Chạy tối đa budgetMs rồi trả {done:false}; gọi lại (reset=false) để tiếp tục từ chỗ cũ.
//   Xong thì trả {done:true, docs:[{name,path,fonts:[{ps,family,style}]}]}.
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
    if (reset || !st) st = g.__fa = { installed: null, di: 0, fi: 0, docs: [] };
    if (!st.installed) {
      st.installed = {};
      var all = app.textFonts;
      for (i = 0; i < all.length; i++) { try { st.installed[all[i].name] = 1; } catch (e) {} }
    }
    while (st.di < app.documents.length) {
      var d = app.documents[st.di];
      if (!st.docs[st.di]) {
        var path = ""; try { path = d.fullName.fsName; } catch (e) {}
        st.docs[st.di] = { name: String(d.name), path: path, fonts: [], seen: {} };
      }
      var rec = st.docs[st.di], frames = d.textFrames;
      while (st.fi < frames.length) {
        if (new Date().getTime() - t0 > budgetMs) return '{"ok":true,"done":false}';
        try {
          var runs = frames[st.fi].textRange.textRuns;
          for (r = 0; r < runs.length && r < 300; r++) {
            var f = runs[r].characterAttributes.textFont, ps = f.name;
            if (!ps || st.installed[ps] || rec.seen[ps]) continue;
            rec.seen[ps] = 1;
            rec.fonts.push({ ps: String(ps), family: String(f.family || ""), style: String(f.style || "") });
          }
        } catch (e) {}
        st.fi++;
      }
      st.di++; st.fi = 0;
    }
    var docs = st.docs;
    g.__fa = null;
    return '{"ok":true,"done":true,"docs":' + faDocsJson(docs) + "}";
  } catch (e) {
    g.__fa = null;
    return '{"ok":false,"error":' + faQ(e) + "}";
  }
}

function faRedraw() { try { app.redraw(); return "ok"; } catch (e) { return "err:" + e; } }
