// FontAtlas startup script @VERSION@ (do Font Atlas cài; gỡ từ Font Atlas → Cài đặt → Tự bật font).
// Nghe afterOpen; font nào của tài liệu KHÔNG ở trạng thái INSTALLED thì hỏi Font Atlas (broker localhost) bật giúp.
// Không sửa tài liệu, không mở hộp thoại; lỗi thì im lặng.
#targetengine "fontatlas"

(function () {
  var CFG = @BROKER_JSON_PATH@;                // đường dẫn broker.json của Font Atlas
  var TIMEOUT_MS = 3000;                        // không giữ InDesign quá 3 s; Font Atlas tải file đám mây chạy nốt ở nền, lần mở sau có ngay

  function readCfg() {
    try {
      var f = File(CFG);
      if (!f.exists) return null;
      f.encoding = "UTF-8"; f.open("r"); var s = f.read(); f.close();
      // KHÔNG eval nội dung tệp: chỉ lấy hai trường đã kiểm dạng (cổng là số, token là 64 ký tự hex).
      var pm = /"port"\s*:\s*(\d{1,5})\b/.exec(s), tm = /"token"\s*:\s*"([0-9a-f]{64})"/.exec(s);
      if (!pm || !tm) return null;
      var port = parseInt(pm[1], 10);
      if (!(port >= 1 && port <= 65535)) return null;
      return { port: port, token: tm[1] };
    } catch (e) { return null; }
  }
  function esc(s) { return String(s).replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/[\u0000-\u001f]/g, " "); }

  function missingFonts(doc) {
    var out = [], seen = {};
    var fonts = doc.fonts.everyItem().getElements();
    for (var i = 0; i < fonts.length; i++) {
      var f = fonts[i], st;
      try { st = f.status; } catch (e) { continue; }
      if (st == FontStatus.INSTALLED) continue;
      var ps = "", fam = "", sty = "", nm = "";
      try { ps = f.postscriptName; } catch (e) {}
      try { fam = f.fontFamily; } catch (e) {}
      try { sty = f.fontStyleName; } catch (e) {}
      try { nm = f.name; } catch (e) {}
      var key = ps || nm;
      if (!key || seen[key]) continue;
      seen[key] = 1;
      out.push({ ps: ps, family: fam, style: sty });
    }
    return out;
  }

  function post(cfg, body) {
    var s = new Socket(); s.timeout = 2;
    if (!s.open("127.0.0.1:" + cfg.port, "BINARY")) return null;
    var bytes = unescape(encodeURIComponent(body));            // UTF-8 thành chuỗi byte
    s.write("POST /v1/ensure HTTP/1.1\r\nHost: 127.0.0.1:" + cfg.port + "\r\nAuthorization: Bearer " + cfg.token +
      "\r\nContent-Type: application/json\r\nContent-Length: " + bytes.length + "\r\nConnection: close\r\n\r\n" + bytes);
    var t0 = new Date().getTime(), reply = "";
    while (new Date().getTime() - t0 < TIMEOUT_MS) {
      var chunk = s.read(); if (chunk) reply += chunk;
      if (!s.connected && !chunk) break;
      $.sleep(30);
    }
    s.close();
    return reply;
  }

  function onAfterOpen(ev) {
    try {
      var doc = (ev.parent && ev.parent.constructor.name == "Document") ? ev.parent : app.activeDocument;
      var miss = missingFonts(doc);
      if (!miss.length) return;
      var cfg = readCfg(); if (!cfg) return;
      var items = [];
      for (var j = 0; j < miss.length; j++)
        items.push('{"ps":"' + esc(miss[j].ps) + '","family":"' + esc(miss[j].family) + '","style":"' + esc(miss[j].style) + '"}');
      post(cfg, '{"v":1,"app":"' + esc(app.name) + '","doc":"' + esc(doc.name) + '","fonts":[' + items.join(",") + ']}');
    } catch (e) {}
  }

  // targetengine bền: chạy lại script thì gỡ listener cũ trước.
  var g = $.global;
  if (g.__fontatlasListeners) { for (var k = 0; k < g.__fontatlasListeners.length; k++) { try { g.__fontatlasListeners[k].remove(); } catch (e) {} } }
  g.__fontatlasListeners = [app.addEventListener("afterOpen", onAfterOpen, false)];
})();
