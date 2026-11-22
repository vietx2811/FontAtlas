// FontAtlas — panel nền của Illustrator. Không hiện giao diện. Mỗi lần tài liệu đổi (sự kiện, hoặc poll dự phòng),
// hỏi host.jsx font thiếu, gửi tới Font Atlas (broker localhost, token trong broker.json) và hỏi lại sau nếu chưa xong.
(function () {
  var cs = window.__adobe_cep__;
  var CFG = null;           // {broker: "/abs/path/broker.json"}
  var last = {};            // khoá tài liệu → chữ ký danh sách font đã gửi
  var retry = {};           // khoá → số lần hỏi lại
  var RETRY_MS = 5000, MAX_RETRY = 6;

  // Log ra tệp để thấy panel chết ở đâu mà KHÔNG cần PlayerDebugMode: `cep-panel.log` cạnh broker.json (thư mục dữ liệu của app).
  // Giới hạn LOG_MAX ký tự (giữ phần cuối). Chỉ ghi mã lỗi, số đếm và TÊN FONT; không bao giờ ghi tên/đường dẫn tài liệu.
  var LOG_MAX = 12000;
  function logPath() {
    var c = CFG; if (!c || !c.broker) return null;
    var i = String(c.broker).lastIndexOf("/");
    return i > 0 ? String(c.broker).substring(0, i) + "/cep-panel.log" : null;
  }
  function log(m) {
    try { cs.evalScript("$.writeln(" + JSON.stringify("[FontAtlas] " + m) + ")"); } catch (e) {}
    try {
      var p = logPath(); if (!p) return;
      var cur = ""; var r = window.cep.fs.readFile(p); if (!r.err && typeof r.data === "string") cur = r.data;
      cur += new Date().toISOString() + " " + String(m).replace(/[\r\n]+/g, " ").substring(0, 400) + "\n";
      if (cur.length > LOG_MAX) { cur = cur.substring(cur.length - LOG_MAX); var nl = cur.indexOf("\n"); if (nl >= 0) cur = cur.substring(nl + 1); }
      window.cep.fs.writeFile(p, cur);
    } catch (e) {}
  }
  var onceKey = {};
  function logOnce(k, m) { if (onceKey[k] === m) return; onceKey[k] = m; log(m); }
  function names(a) { return (a || []).map(function (f) { return typeof f === "string" ? f : (f && (f.ps || f.name)) || "?"; }).join(","); }
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
    var c = config(); if (!c || !c.broker) { logOnce("cfg", "không đọc được fontatlas.json"); return null; }
    var b = readJson(c.broker);              // đọc lại mỗi lần: cổng và token đổi mỗi lần Font Atlas khởi động
    // như script InDesign: cổng 1..65535 và token đúng dạng (64 hex) — tệp lạ/hỏng thì coi như chưa có broker
    var port = b && Number(b.port);
    if (!b) { logOnce("broker", "chưa có broker.json (Font Atlas chưa chạy?)"); return null; }
    if (!(port >= 1 && port <= 65535 && Math.floor(port) === port)) { logOnce("broker", "broker.json: cổng không hợp lệ"); return null; }
    if (typeof b.token !== "string" || !/^[0-9a-f]{64}$/.test(b.token)) { logOnce("broker", "broker.json: token sai dạng"); return null; }
    logOnce("broker", "broker.json ok");
    return { port: port, token: b.token };
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
    if (!doc.fonts.length) { delete last[key]; delete retry[key]; return; }  // không font thiếu (đã ghi trong log quét)
    var sig = doc.fonts.map(function (f) { return f.ps; }).sort().join("|");
    if (last[key] === sig) return;
    var b = broker(); if (!b) { lastSig = null; return; }       // Font Atlas chưa chạy: lượt poll sau PHẢI quét lại (lastSig=null), không được coi là đã xử lý
    log("gửi ensure: font thiếu=" + names(doc.fonts));
    post(b, { v: 1, app: "Adobe Illustrator", doc: doc.name, fonts: doc.fonts }, function (status, r) {
      if (status !== 200 || !r) { log("ensure lỗi http=" + status); return; }
      var again = r.results.some(function (x) { return x.status === "busy" || x.status === "not_downloaded" || x.status === "failed"; });
      var n = (retry[key] || 0) + 1;
      if (again && n <= MAX_RETRY) { retry[key] = n; setTimeout(function () { delete last[key]; trigger(true); }, RETRY_MS); }
      last[key] = sig;
      if (r.found && r.found.length) cs.evalScript("faRedraw()");
      log("ensure xong: bật=" + names(r.found) + " chưa có=" + names(r.missing));
    });
  }

  // ---- quét ----
  // Dựa vào SỰ KIỆN tài liệu; poll chỉ là dự phòng thưa (POLL_MS) và chỉ hỏi chữ ký rẻ `faSig()`: tài liệu không đổi thì
  // không quét gì cả. Quét thật đi theo lát ngắn (SLICE_MS) để không giữ luồng chính của Illustrator.
  var lastSig = null, scanning = false, again = false, timer = null, hostLoaded = false, sigN = 0;
  var SLICE_MS = 700, POLL_MS = 20000;

  function logScan(r) {
    var s = r.stats; if (!s) { log("quét xong: " + r.docs.length + " tài liệu (host cũ, không có stats)"); return; }
    log("quét xong: tài liệu=" + s.docs + " frame=" + s.frames + " run=" + s.runs + " runKhôngFont=" + s.noFont + " lỗi=" + s.errs + " fontCài=" + s.installed +
      " thấy=[" + (s.seen || []).map(function (x) { return x.ps + (x.miss ? "(thiếu)" : "(có)"); }).join(",") + "]" + ((s.errMsgs || []).length ? " lỗiMẫu=" + s.errMsgs.join(" | ") : ""));
  }

  function runScan(sig) {
    scanning = true;
    var reset = true;
    (function step() {
      cs.evalScript("faScan(" + reset + "," + SLICE_MS + ")", function (res) {
        reset = false;
        var r = null; try { r = JSON.parse(res); } catch (e) {}
        if (!r || !r.ok) { scanning = false; log("quét lỗi: " + (r ? "host báo " + r.error : "phản hồi không phải JSON, bắt đầu '" + String(res).substring(0, 40) + "'")); return; }
        if (!r.done) { setTimeout(step, 250); return; }        // nhả luồng chính giữa hai lát
        lastSig = sig;
        logScan(r);
        r.docs.forEach(handle);
        scanning = false;
        if (again) { again = false; trigger(true); }
      });
    })();
  }

  function trigger(force) {
    if (scanning) { if (force) again = true; return; }
    cs.evalScript("faSig()", function (sig) {
      if (sig === "EvalScript error." || sig === "undefined" || sig === "err") {
        // host.jsx chưa nạp (ScriptPath không chạy cho panel nền?) hoặc faSig lỗi: nạp tay MỘT lần rồi thử lại.
        log("faSig lỗi: '" + String(sig).substring(0, 40) + "'" + (hostLoaded ? "" : "; nạp host.jsx bằng tay"));
        if (!hostLoaded) { hostLoaded = true; cs.evalScript('$.evalFile(' + JSON.stringify(here() + "/jsx/host.jsx") + ')', function (x) { log("evalFile host.jsx → '" + String(x).substring(0, 40) + "'"); trigger(true); }); }
        return;
      }
      if (!force && sig === lastSig) return;
      sigN++; if (sigN <= 3 || sig !== lastSig) log("tập tài liệu: " + (sig ? sig.split("\n").length : 0) + " tài liệu, quét");
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
  config(); log("panel nền đã nạp (log ở " + (logPath() || "không có đường dẫn") + ")");
})();
