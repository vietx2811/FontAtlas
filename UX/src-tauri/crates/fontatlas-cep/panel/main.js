// FontAtlas — panel nền của Illustrator. Không hiện giao diện. Mỗi lần tài liệu đổi (sự kiện, hoặc poll dự phòng),
// hỏi host.jsx font thiếu, gửi tới Font Atlas (broker localhost, token trong broker.json) và hỏi lại sau nếu chưa xong.
(function () {
  var cs = window.__adobe_cep__;
  var CFG = null;           // {broker: "/abs/path/broker.json"}
  var last = {};            // khoá tài liệu → chữ ký danh sách font đã gửi
  var retry = {};           // khoá → số lần hỏi lại
  var POLL_MS = 3000, RETRY_MS = 5000, MAX_RETRY = 6;

  function log(m) { try { cs.evalScript("$.writeln(" + JSON.stringify("[FontAtlas] " + m) + ")"); } catch (e) {} }
  function readJson(p) {
    var r = window.cep.fs.readFile(p);
    if (r.err) return null;
    try { return JSON.parse(r.data); } catch (e) { return null; }
  }
  function here() {
    // đường dẫn thư mục của panel (…/com.fontatlas.autoactivate)
    var u = decodeURI(cs.getSystemPath("extension")).replace(/^file:\/\//, "");
    return u;
  }
  function config() {
    if (!CFG) CFG = readJson(here() + "/fontatlas.json");
    return CFG;
  }
  function broker() {
    var c = config(); if (!c || !c.broker) return null;
    var b = readJson(c.broker);              // đọc lại mỗi lần: cổng và token đổi mỗi lần Font Atlas khởi động
    return b && b.port && b.token ? b : null;
  }
  function post(b, body, done) {
    var x = new XMLHttpRequest(); x.timeout = 12000;
    x.open("POST", "http://127.0.0.1:" + b.port + "/v1/ensure");
    x.setRequestHeader("Authorization", "Bearer " + b.token);
    x.setRequestHeader("Content-Type", "application/json");
    x.onload = function () { var r = null; try { r = JSON.parse(x.responseText); } catch (e) {} done(x.status, r); };
    x.onerror = x.ontimeout = function () { done(0, null); };
    x.send(JSON.stringify(body));
  }

  function handle(doc) {
    var key = doc.path || doc.name;
    if (!doc.fonts.length) { delete last[key]; delete retry[key]; return; }
    var sig = doc.fonts.map(function (f) { return f.ps; }).sort().join("|");
    if (last[key] === sig) return;
    var b = broker(); if (!b) return;       // Font Atlas chưa chạy: hỏi lại ở lượt sau, không nhớ
    post(b, { v: 1, app: "Adobe Illustrator", doc: doc.name, fonts: doc.fonts }, function (status, r) {
      if (status !== 200 || !r) { log("broker " + status); return; }
      var again = r.results.some(function (x) { return x.status === "busy" || x.status === "not_downloaded" || x.status === "failed"; });
      var n = (retry[key] || 0) + 1;
      if (again && n <= MAX_RETRY) { retry[key] = n; setTimeout(function () { delete last[key]; trigger(true); }, RETRY_MS); }
      last[key] = sig;
      if (r.found && r.found.length) cs.evalScript("faRedraw()");
      log(doc.name + ": " + (r.found || []).length + " font bật, " + (r.missing || []).length + " chưa có");
    });
  }

  // ---- quét ----
  // Dựa vào SỰ KIỆN tài liệu; poll chỉ là dự phòng thưa (POLL_MS) và chỉ hỏi chữ ký rẻ `faSig()`: tài liệu không đổi thì
  // không quét gì cả. Quét thật đi theo lát ngắn (SLICE_MS) để không giữ luồng chính của Illustrator.
  var lastSig = null, scanning = false, again = false, timer = null;
  var SLICE_MS = 700, POLL_MS = 20000;

  function runScan(sig) {
    scanning = true;
    var reset = true;
    (function step() {
      cs.evalScript("faScan(" + reset + "," + SLICE_MS + ")", function (res) {
        reset = false;
        var r = null; try { r = JSON.parse(res); } catch (e) {}
        if (!r || !r.ok) { scanning = false; return; }
        if (!r.done) { setTimeout(step, 250); return; }        // nhả luồng chính giữa hai lát
        lastSig = sig;
        r.docs.forEach(handle);
        scanning = false;
        if (again) { again = false; trigger(true); }
      });
    })();
  }

  function trigger(force) {
    if (scanning) { if (force) again = true; return; }
    cs.evalScript("faSig()", function (sig) {
      if (sig === "err" || (!force && sig === lastSig)) return;
      runScan(sig);
    });
  }

  function onEvent() {                                          // gộp chuỗi sự kiện dồn dập (mở tài liệu bắn nhiều sự kiện)
    if (timer) clearTimeout(timer);
    timer = setTimeout(function () { timer = null; trigger(true); }, 400);
  }

  ["documentAfterActivate", "documentCreated", "documentOpened", "com.adobe.csxs.events.documentAfterActivate",
   "com.adobe.csxs.events.ApplicationActivate", "applicationActivate"].forEach(function (n) {
    try { cs.addEventListener(n, onEvent); } catch (e) {}
  });
  setInterval(function () { trigger(false); }, POLL_MS);
  trigger(true);
  log("panel nền đã nạp");
})();
