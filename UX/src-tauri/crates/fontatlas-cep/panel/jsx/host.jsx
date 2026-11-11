// FontAtlas — phần chạy trong Illustrator (ExtendScript), gọi từ panel nền qua evalScript.
// faFonts(): JSON { docs: [ { name, path, fonts: [ { ps, family, style } ] } ] } — chỉ font KHÔNG có trong danh sách
// font đã cài của Illustrator (app.textFonts). Không sửa tài liệu, không mở hộp thoại.
function faFonts() {
  try {
    var installed = {}, all = app.textFonts, i, n;
    for (i = 0; i < all.length; i++) { try { installed[all[i].name] = 1; } catch (e) {} }
    var out = [];
    for (i = 0; i < app.documents.length; i++) {
      var d = app.documents[i], seen = {}, fonts = [], path = "";
      try { path = d.fullName.fsName; } catch (e) {}
      var frames = d.textFrames;
      for (n = 0; n < frames.length && n < 5000; n++) {
        try {
          var runs = frames[n].textRange.characterAttributes;
          var f = runs.textFont;
          var ps = f.name;
          if (!ps || installed[ps] || seen[ps]) continue;
          seen[ps] = 1;
          fonts.push({ ps: String(ps), family: String(f.family || ""), style: String(f.style || "") });
        } catch (e) {}
      }
      out.push({ name: String(d.name), path: path, fonts: fonts });
    }
    return JSON.stringify({ ok: true, docs: out });
  } catch (e) {
    return JSON.stringify({ ok: false, error: String(e) });
  }
}
function faRedraw() { try { app.redraw(); return "ok"; } catch (e) { return "err:" + e; } }
