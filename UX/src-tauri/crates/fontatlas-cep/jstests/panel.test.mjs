// Test panel CEP + host.jsx KHÔNG cần Illustrator: chạy host.jsx (ExtendScript) và main.js (panel) trong vm của Node,
// với `app` giả (tài liệu giả có textFont.name thiếu / ném lỗi / bị thay thế) và đồng hồ giả để thử vòng poll.
// Chạy: node --test crates/fontatlas-cep/jstests   (cargo test -p fontatlas-cep cũng chạy nó nếu có `node`).
import test from "node:test";
import assert from "node:assert/strict";
import vm from "node:vm";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const HOST = fs.readFileSync(path.join(here, "../panel/jsx/host.jsx"), "utf8");
const MAIN = fs.readFileSync(path.join(here, "../panel/main.js"), "utf8");
const TOKEN = "a".repeat(64);
const BROKER = "/data/broker.json";
const LOG = "/data/cep-panel.log";
const SECRET_DOC = "Khách-hàng-BÍ-MẬT.ai";

const run = (name, ps, fam = "F", sty = "Regular") => ({ characterAttributes: { textFont: { name: ps, family: fam, style: sty } }, _n: name });
const frameOf = (...runs) => ({ textRange: { textRuns: runs } });
const doc = (frames, name = SECRET_DOC) => ({ name, fullName: { fsName: "/Users/x/" + name }, textFrames: frames });

// Môi trường Illustrator giả: app + $ trong một vm; evalScript chạy chuỗi trong vm đó.
function makeHost({ installed = ["ArialMT"], docs = [], loaded = true } = {}) {
  const app = { textFonts: installed.map((n) => ({ name: n })), documents: docs };
  const ctx = vm.createContext({ app, $: { global: {}, writeln() {} }, Date });
  const st = { loaded, calls: [] };
  if (loaded) vm.runInContext(HOST, ctx);
  const evalScript = (src, cb) => {
    st.calls.push(src);
    let r;
    if (src.startsWith("$.evalFile(")) { vm.runInContext(HOST, ctx); st.loaded = true; r = ""; }
    else if (!st.loaded) r = "EvalScript error.";
    else { try { r = vm.runInContext(src, ctx); } catch (e) { r = "EvalScript error."; } }
    if (cb) setImmediate(() => cb(String(r)));
  };
  return { app, ctx, st, evalScript };
}

// Panel giả: main.js chạy trong vm với CEP/XHR/đồng hồ giả.
function makePanel(host, { broker = { port: 4321, token: TOKEN }, respond } = {}) {
  const files = new Map([[HERE_CFG, JSON.stringify({ broker: BROKER })]]);
  if (broker) files.set(BROKER, JSON.stringify(broker));
  const posts = [], timers = [], listeners = {};
  let now = 0;
  class XHR {
    open(m, url) { this.m = m; this.url = url; this.h = {}; }
    setRequestHeader(k, v) { this.h[k] = v; }
    send(body) {
      const b = JSON.parse(body); posts.push({ url: this.url, auth: this.h.Authorization, body: b });
      const out = (respond ? respond(b, posts.length) : { status: 200, json: { ok: true, results: b.fonts.map((f) => ({ name: f.ps, status: "activated" })), found: b.fonts.map((f) => f.ps), missing: [] } });
      setImmediate(() => { this.status = out.status; this.responseText = JSON.stringify(out.json); this.onload && this.onload(); });
    }
  }
  const window = {
    __adobe_cep__: { evalScript: host.evalScript, addEventListener: (n, f) => { (listeners[n] ||= []).push(f); }, getSystemPath: () => "file://" + HERE_EXT.replace(/ /g, "%20") },
    cep: { fs: {
      readFile: (p) => (files.has(p) ? { err: 0, data: files.get(p) } : { err: 2 }),
      writeFile: (p, d) => { files.set(p, d); return { err: 0 }; },
    } },
  };
  const sandbox = {
    window, JSON, Date: class extends Date { constructor(...a) { super(...(a.length ? a : [now])); } static now() { return now; } },
    XMLHttpRequest: XHR, decodeURI, Math, Number, String, Array, RegExp, console,
    setTimeout: (f, ms) => { const t = { f, at: now + ms, id: timers.length + 1, iv: 0 }; timers.push(t); return t.id; },
    clearTimeout: (id) => { const t = timers.find((x) => x.id === id); if (t) t.f = null; },
    setInterval: (f, ms) => { const t = { f, at: now + ms, id: timers.length + 1, iv: ms }; timers.push(t); return t.id; },
  };
  vm.runInContext(MAIN, vm.createContext(sandbox));
  const flush = async () => { for (let i = 0; i < 40; i++) await new Promise((r) => setImmediate(r)); };
  const advance = async (ms) => {
    const end = now + ms;
    for (;;) {
      await flush();
      const due = timers.filter((t) => t.f && t.at <= end).sort((a, b) => a.at - b.at)[0];
      if (!due) break;
      now = due.at; if (due.iv) due.at += due.iv; else { const f = due.f; due.f = null; f(); continue; } due.f();
    }
    now = end; await flush();
  };
  return { posts, files, listeners, advance, flush, log: () => files.get(LOG) || "" };
}
const HERE_EXT = "/Users/x/Library/Application Support/Adobe/CEP/extensions/com.fontatlas.autoactivate";
const HERE_CFG = HERE_EXT + "/fontatlas.json";
const scanJson = (host) => JSON.parse(vm.runInContext("faScan(true, 1000)", host.ctx));

test("font thiếu (không có trong app.textFonts) được gửi tới broker, log có tên font, KHÔNG có tên tài liệu", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "DMSerifDisplay-Regular", "DM Serif Display"))])] });
  const p = makePanel(host); await p.advance(1000);
  assert.equal(p.posts.length, 1);
  assert.equal(p.posts[0].body.app, "Adobe Illustrator");
  assert.deepEqual(p.posts[0].body.fonts.map((f) => f.ps), ["DMSerifDisplay-Regular"]);
  assert.equal(p.posts[0].auth, "Bearer " + TOKEN);
  assert.equal(p.posts[0].body.fonts[0].miss, true);
  assert.match(p.log(), /gửi ensure: font=1 \(Illustrator báo thiếu=DMSerifDisplay-Regular\)/);
  assert.match(p.log(), /ensure xong: vừa bật=DMSerifDisplay-Regular/);
  assert.ok(!p.log().includes("BÍ-MẬT") && !p.log().includes("Khách") && !p.log().includes("/Users/x/"), "log không được chứa tên/đường dẫn tài liệu");
});

test("mỗi text run có font riêng: font thứ hai trong cùng frame cũng được gửi", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "ArialMT"), run("b", "Chonburi-Regular"), run("c", "Chonburi-Regular"))])] });
  const p = makePanel(host); await p.advance(1000);
  assert.deepEqual(p.posts[0].body.fonts.map((f) => f.ps).sort(), ["ArialMT", "Chonburi-Regular"]);
  assert.deepEqual(p.posts[0].body.fonts.map((f) => [f.ps, f.miss]).sort(), [["ArialMT", false], ["Chonburi-Regular", true]]);
});

test("font có trong app.textFonts VẪN được gửi (miss=false): textFonts có thể còn giữ font Font Atlas đã tắt; broker quyết định", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "ArialMT"))])] });
  const p = makePanel(host); await p.advance(1000);
  assert.equal(p.posts.length, 1);
  assert.deepEqual(p.posts[0].body.fonts.map((f) => [f.ps, f.miss]), [["ArialMT", false]]);
  assert.match(p.log(), /thấy=\[ArialMT\(có\)\]/);
  assert.match(p.log(), /gửi ensure: font=1 \(Illustrator báo thiếu=\)/);
});

test("font 'có' trong textFonts nhưng broker báo vừa bật (activated) → faRedraw; log không liệt kê font hệ thống là 'chưa có'", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "Chonburi-Regular"), run("b", "ArialMT"))])], installed: ["ArialMT", "Chonburi-Regular"] });
  const p = makePanel(host, { respond: (b) => ({ status: 200, json: { ok: true, results: [{ name: "Chonburi-Regular", status: "activated" }, { name: "ArialMT", status: "missing" }], found: ["Chonburi-Regular"], missing: ["ArialMT"] } }) });
  await p.advance(1000);
  assert.ok(host.st.calls.includes("faRedraw()"));
  assert.match(p.log(), /vừa bật=Chonburi-Regular/);
  assert.match(p.log(), /chưa có\(Illustrator báo thiếu\)=\s*$/m);
});

test("nhiều hơn 256 font → chia lô 256, mỗi lô một yêu cầu", async () => {
  const runs = Array.from({ length: 300 }, (_, i) => run("r" + i, "Font-" + i));
  const host = makeHost({ docs: [doc([frameOf(...runs)])] });
  const p = makePanel(host); await p.advance(2000);
  assert.deepEqual(p.posts.map((x) => x.body.fonts.length), [256, 44]);
});

test("ensure lỗi http: KHÔNG đánh dấu đã gửi → lượt sau gửi lại", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "Chonburi-Regular"))])] });
  const p = makePanel(host, { respond: (b, n) => (n === 1 ? { status: 500, json: {} } : { status: 200, json: { ok: true, results: [], found: [], missing: [] } }) });
  await p.advance(1000);
  assert.equal(p.posts.length, 1);
  await p.advance(30000);
  assert.ok(p.posts.length >= 2, "phải gửi lại sau lỗi: " + p.posts.length);
});

test("tài liệu IMPORT (SVG/PDF): Illustrator thay font bằng font cài sẵn → không có font thiếu (miss=false), log lộ rõ chuyện bị thay thế", async () => {
  // Đây là chẩn đoán cho lần thử thật: nếu log ghi thấy=[ArialMT(có)] dù chữ thiết kế bằng DM Serif thì textFont KHÔNG giữ tên gốc.
  const host = makeHost({ docs: [doc([frameOf(run("svg", "ArialMT"))], "import.svg")] });
  const p = makePanel(host); await p.advance(1000);
  assert.deepEqual(p.posts.map((x) => x.body.fonts.map((f) => f.miss)), [[false]]);
  assert.match(p.log(), /runKhôngFont=0/);
  assert.match(p.log(), /thấy=\[ArialMT\(có\)\]/);
});

test("textFont ném lỗi khi font thiếu: KHÔNG còn bị nuốt im lặng, log ghi lỗi mẫu và số lỗi", async () => {
  const bad = { characterAttributes: {} };
  Object.defineProperty(bad.characterAttributes, "textFont", { get() { throw new Error("Missing font 1346458189 PARM"); } });
  const host = makeHost({ docs: [doc([frameOf(bad, bad)])] });
  const r = scanJson(host);
  assert.equal(r.ok, true); assert.equal(r.stats.errs, 2); assert.equal(r.stats.runs, 2);
  assert.match(r.stats.errMsgs[0], /^textFont:Missing font/);
  const p = makePanel(makeHost({ docs: [doc([frameOf(bad)])] })); await p.advance(1000);
  assert.equal(p.posts.length, 0);
  assert.match(p.log(), /lỗi=1/); assert.match(p.log(), /lỗiMẫu=textFont:Missing font/);
});

test("textFont null (run không có font) và textFrames/textRuns ném lỗi: đếm và ghi, không làm sập quét", () => {
  const nullRun = { characterAttributes: { textFont: null } };
  const d1 = doc([frameOf(nullRun)]);
  const d2 = { name: "x", fullName: { fsName: "/x" }, get textFrames() { throw new Error("no frames"); } };
  const d3 = doc([{ get textRange() { throw new Error("no range"); } }]);
  const r = scanJson(makeHost({ docs: [d1, d2, d3] }));
  assert.equal(r.ok, true); assert.equal(r.stats.noFont, 1); assert.equal(r.stats.docs, 3);
  assert.ok(r.stats.errMsgs.some((m) => m.startsWith("frames:")) && r.stats.errMsgs.some((m) => m.startsWith("range:")), JSON.stringify(r.stats.errMsgs));
});

test("quét theo lát: hết ngân sách thì trả done:false, gọi tiếp thì xong, không bỏ sót run nào", () => {
  const many = Array.from({ length: 5 }, (_, i) => run("r" + i, "F" + i));
  const host = makeHost({ docs: [doc([frameOf(...many)])] });
  let r = JSON.parse(vm.runInContext("faScan(true, -1)", host.ctx)); // ngân sách âm → dừng ngay
  assert.equal(r.done, false);
  let guard = 0; while (!r.done && guard++ < 50) r = JSON.parse(vm.runInContext("faScan(false, 0)", host.ctx));
  assert.equal(r.done, true);
  assert.equal(r.docs[0].fonts.length, 5);
  assert.equal(r.stats.frames, 1); assert.equal(r.stats.runs, 5);
});

test("VÒNG POLL: không có sự kiện nào, tài liệu mở sau khi panel nạp → poll 20 s thấy chữ ký đổi, quét và gửi", async () => {
  const host = makeHost({ docs: [] });
  const p = makePanel(host); await p.advance(1000);
  assert.equal(p.posts.length, 0);
  host.app.documents.push(doc([frameOf(run("a", "DMSerifDisplay-Regular"))]));
  await p.advance(18000); assert.equal(p.posts.length, 0, "chưa tới mốc poll 20 s");
  await p.advance(3000);
  assert.equal(p.posts.length, 1);
});

test("poll không quét lại khi chữ ký tài liệu không đổi (rẻ)", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "ArialMT"))])] });
  const p = makePanel(host); await p.advance(1000);
  const scans = () => host.st.calls.filter((c) => c.startsWith("faScan(")).length;
  const before = scans(); await p.advance(65000);
  assert.equal(scans(), before);
});

test("sự kiện tài liệu (documentAfterActivate…) gộp 400 ms rồi quét ngay, không đợi poll", async () => {
  const host = makeHost({ docs: [] });
  const p = makePanel(host); await p.advance(1000);
  host.app.documents.push(doc([frameOf(run("a", "Chonburi-Regular"))]));
  p.listeners.documentOpened.forEach((f) => f()); p.listeners.documentAfterActivate.forEach((f) => f());
  await p.advance(500);
  assert.equal(p.posts.length, 1);
});

test("Font Atlas chưa chạy (chưa có broker.json): không gửi, log nói rõ; broker xuất hiện sau thì poll kế tiếp VẪN gửi (không kẹt)", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "DMSerifDisplay-Regular"))])] });
  const p = makePanel(host, { broker: null }); await p.advance(1000);
  assert.equal(p.posts.length, 0);
  assert.match(p.log(), /chưa có broker\.json/);
  p.files.set(BROKER, JSON.stringify({ port: 5555, token: TOKEN }));
  await p.advance(21000);
  assert.equal(p.posts.length, 1, "lỗi cũ: lastSig đã đặt nên poll không bao giờ quét lại");
  assert.ok(p.posts[0].url.includes(":5555"));
});

test("broker.json hỏng (cổng/token sai dạng): không gửi, log nêu lý do", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "DMSerifDisplay-Regular"))])] });
  const p = makePanel(host, { broker: { port: 99999, token: TOKEN } }); await p.advance(1000);
  assert.equal(p.posts.length, 0); assert.match(p.log(), /cổng không hợp lệ/);
  const q = makePanel(makeHost({ docs: [doc([frameOf(run("a", "DMSerifDisplay-Regular"))])] }), { broker: { port: 1234, token: "xyz" } }); await q.advance(1000);
  assert.equal(q.posts.length, 0); assert.match(q.log(), /token sai dạng/);
});

test("host.jsx chưa được nạp (ScriptPath không chạy cho panel nền): panel nạp tay bằng $.evalFile rồi tiếp tục", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "DMSerifDisplay-Regular"))])], loaded: false });
  const p = makePanel(host); await p.advance(2000);
  assert.ok(host.st.calls.some((c) => c.startsWith("$.evalFile(") && c.includes("/jsx/host.jsx")));
  assert.equal(p.posts.length, 1);
  assert.match(p.log(), /faSig lỗi/); assert.match(p.log(), /nạp host\.jsx bằng tay/);
});

test("host.jsx hỏng hẳn: không lặp vô hạn, nạp tay tối đa 3 lần", async () => {
  const host = makeHost({ docs: [], loaded: false });
  host.evalScript = (src, cb) => { host.st.calls.push(src); if (cb) setImmediate(() => cb("EvalScript error.")); };
  const p = makePanel(host); await p.advance(70000);
  assert.equal(host.st.calls.filter((c) => c.startsWith("$.evalFile(")).length, 3);
});

test("ensure trả busy/not_downloaded: hỏi lại sau 5 s, tối đa 6 lần", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "DMSerifDisplay-Regular"))])] });
  const p = makePanel(host, { respond: (b) => ({ status: 200, json: { ok: true, results: b.fonts.map((f) => ({ name: f.ps, status: "busy" })), found: [], missing: [] } }) });
  await p.advance(60000);
  assert.equal(p.posts.length, 7, "1 lần đầu + 6 lần hỏi lại");
});

test("ensure thành công có font bật thì panel gọi faRedraw()", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "DMSerifDisplay-Regular"))])] });
  const p = makePanel(host); await p.advance(1000);
  assert.ok(host.st.calls.includes("faRedraw()"));
});

test("log bị chặn kích thước, giữ phần cuối, cắt ở ranh giới dòng", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "DMSerifDisplay-Regular"))])] });
  const p = makePanel(host, { respond: () => ({ status: 500, json: {} }) });
  await p.advance(200000);
  assert.ok(p.log().length <= 12000, String(p.log().length));
  assert.ok(p.log().endsWith("\n"));
  assert.match(p.log().split("\n")[0], /^\d{4}-\d\d-\d\dT/, "dòng đầu còn nguyên, không bị cắt giữa chừng");
});

// Illustrator 2027 thật (log lần thử 2026-11-23): `frame=1 run=0` — textRuns không liệt kê được gì dù frame có chữ, nên panel không
// bao giờ thấy font nào. Các test dưới mô phỏng đúng hiện tượng đó: textRuns rỗng / length không phải số.
const para = (ps) => ({ characterAttributes: { textFont: { name: ps, family: "F", style: "Regular" } } });
const frame2027 = (textRuns, paragraphs, whole) => ({ textRange: { textRuns, paragraphs, characterAttributes: whole ? { textFont: { name: whole, family: "F", style: "Regular" } } : {} } });

test("2027: textRuns rỗng (length 0) → đọc từng paragraph, thấy font thiếu và gửi", async () => {
  const host = makeHost({ docs: [doc([frame2027([], [para("DMSerifDisplay-Regular"), para("ArialMT")])])] });
  const r = scanJson(host);
  assert.equal(r.stats.viaParas, 2); assert.equal(r.stats.viaRuns, 0);
  assert.match(r.stats.probe, /runs=0 paras=2 via=paras/);
  const p = makePanel(makeHost({ docs: [doc([frame2027([], [para("DMSerifDisplay-Regular")])])] })); await p.advance(1000);
  assert.deepEqual(p.posts[0].body.fonts.map((f) => f.ps), ["DMSerifDisplay-Regular"]);
  assert.match(p.log(), /qua\[run\/đoạn\/frame\/ký tự\]=0\/1\/0\/0/);
});

test("2027: textRuns.length không phải số (undefined) và không có paragraph → đọc font của cả textRange", async () => {
  const host = makeHost({ docs: [doc([frame2027({}, undefined, "Chonburi-Regular")])] });
  const r = scanJson(host);
  assert.equal(r.stats.viaFrame, 1);
  assert.match(r.stats.probe, /via=frame/);
  const p = makePanel(makeHost({ docs: [doc([frame2027({}, undefined, "Chonburi-Regular")])] })); await p.advance(1000);
  assert.deepEqual(p.posts[0].body.fonts.map((f) => f.ps), ["Chonburi-Regular"]);
});

test("textRuns hoạt động bình thường thì KHÔNG dùng đường dự phòng", () => {
  const r = scanJson(makeHost({ docs: [doc([frameOf(run("a", "ArialMT"))])] }));
  assert.equal(r.stats.viaRuns, 1); assert.equal(r.stats.viaParas, 0); assert.equal(r.stats.viaFrame, 0);
});

test("faSig lỗi tạm ('err' lúc có hộp modal) KHÔNG tính vào số lần nạp host.jsx; sau đó vẫn nạp được khi host thật sự chưa nạp", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "DMSerifDisplay-Regular"))])] });
  let n = 0; const real = host.evalScript;
  host.evalScript = (src, cb) => { if (src === "faSig()" && n++ < 2) { host.st.calls.push(src); return setImmediate(() => cb("err")); } real(src, cb); };
  const p = makePanel(host); await p.advance(50000);
  assert.equal(host.st.calls.filter((c) => c.startsWith("$.evalFile(")).length, 0, "err tạm không được ăn lượt nạp tay");
  assert.equal(p.posts.length, 1, "poll sau đó chạy lại bình thường");
});

test("Font Atlas khởi động lại (cổng/token đổi): tài liệu đang mở được hỏi lại ngay cả khi tập tài liệu không đổi", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "DMSerifDisplay-Regular"))])] });
  const p = makePanel(host); await p.advance(1000);
  assert.equal(p.posts.length, 1);
  p.files.set(BROKER, JSON.stringify({ port: 7777, token: "b".repeat(64) }));
  await p.advance(21000);
  assert.equal(p.posts.length, 2, "phải hỏi lại sau khi broker đổi");
  assert.ok(p.posts[1].url.includes(":7777") && p.posts[1].auth === "Bearer " + "b".repeat(64));
  assert.match(p.log(), /broker đổi/);
  await p.advance(65000);
  assert.equal(p.posts.length, 2, "không hỏi lại khi broker không đổi nữa");
});

test("broker vắng: poll KHÔNG quét lại mỗi 20 s và không ghi log lặp; broker.json xuất hiện thì quét ngay", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "DMSerifDisplay-Regular"))])] });
  const p = makePanel(host, { broker: null }); await p.advance(1000);
  const scans = () => host.st.calls.filter((c) => c.startsWith("faScan(")).length;
  const logBefore = p.log();
  await p.advance(180000);
  assert.equal(scans(), 0, "không có chỗ gửi thì không quét");
  assert.equal(p.log(), logBefore, "không thêm dòng log nào trong lúc vắng");
  p.files.set(BROKER, JSON.stringify({ port: 4321, token: TOKEN }));
  await p.advance(21000);
  assert.equal(p.posts.length, 1);
});

test("log 'quét xong' chỉ ghi khi kết quả khác lần trước", async () => {
  const host = makeHost({ docs: [doc([frameOf(run("a", "ArialMT"))])] });
  const p = makePanel(host); await p.advance(1000);
  for (let i = 0; i < 4; i++) { p.listeners.documentOpened.forEach((f) => f()); await p.advance(600); }
  assert.equal((p.log().match(/quét xong/g) || []).length, 1);
});

// Đoạn TRỘN font: Illustrator không trả font đơn cho đoạn (undefined hoặc ném lỗi) → phải lấy mẫu từng ký tự, không sót font thứ hai.
const ch = (ps) => ({ characterAttributes: { textFont: { name: ps, family: "F", style: "R" } } });
const mixedPara = (chars, mode) => {
  const ca = {};
  if (mode === "throw") Object.defineProperty(ca, "textFont", { get() { throw new Error("mixed"); } }); else if (mode === "first") ca.textFont = chars[0].characterAttributes.textFont; else ca.textFont = undefined;
  return { characterAttributes: ca, characters: chars };
};

test("đoạn trộn font (textFont undefined) → lấy mẫu ký tự, thấy cả hai font", async () => {
  const chars = [ch("ArialMT"), ch("ArialMT"), ch("Chonburi-Regular"), ch("ArialMT")];
  const r = scanJson(makeHost({ docs: [doc([frame2027([], [mixedPara(chars, "undef")])])] }));
  assert.equal(r.stats.viaChars, 1);
  assert.deepEqual(r.docs[0].fonts.map((f) => f.ps), ["Chonburi-Regular"]);
  const p = makePanel(makeHost({ docs: [doc([frame2027([], [mixedPara(chars, "undef")])])] })); await p.advance(1000);
  assert.deepEqual(p.posts[0].body.fonts.map((f) => f.ps).sort(), ["ArialMT", "Chonburi-Regular"]);
});

test("đoạn trộn font mà textFont ném lỗi → vẫn lấy mẫu ký tự, không sập", () => {
  const chars = [ch("DMSerifDisplay-Regular"), ch("ArialMT")];
  const r = scanJson(makeHost({ docs: [doc([frame2027([], [mixedPara(chars, "throw")])])] }));
  assert.equal(r.ok, true);
  assert.deepEqual(r.docs[0].fonts.map((f) => f.ps), ["DMSerifDisplay-Regular"]);
});

test("đoạn dài: lấy mẫu có trần (~200 lần đọc) nhưng luôn gồm ký tự cuối", () => {
  const n = 5000; let reads = 0;
  const chars = Array.from({ length: n }, (_, i) => ({ get characterAttributes() { reads++; return { textFont: { name: i === n - 1 ? "Chonburi-Regular" : "ArialMT", family: "F", style: "R" } }; } }));
  const r = scanJson(makeHost({ docs: [doc([frame2027([], [mixedPara(chars, "undef")])])] }));
  assert.ok(reads <= 260, "số lần đọc: " + reads);
  assert.deepEqual(r.docs[0].fonts.map((f) => f.ps), ["Chonburi-Regular"]);
});

// Illustrator 2027 thật: đoạn trộn 3 font trả font ĐẦU TIÊN (không undefined) → phải vẫn lấy mẫu ký tự, thấy font 2 và 3.
test("đoạn trộn font trả font đầu tiên (hành vi Illustrator 2027) → vẫn thấy font thứ 2, 3", () => {
  const chars = [ch("Chonburi-Regular"), ch("Chonburi-Regular"), ch("DMSerifDisplay-Regular"), ch("ArialMT")];
  const r = scanJson(makeHost({ docs: [doc([frame2027([], [mixedPara(chars, "first")])])], installed: ["ArialMT"] }));
  assert.deepEqual(r.docs[0].fonts.map((f) => f.ps).sort(), ["Chonburi-Regular", "DMSerifDisplay-Regular"]);
  assert.equal(r.stats.viaChars, 1);
});

// Lấy mẫu thưa rồi dày; bỏ khoảng trắng; đoạn đồng nhất không tốn lần đọc dày và không tính viaChars.
const bigPara = (n, fontAt, textAt) => {
  let reads = 0;
  const chars = Array.from({ length: n }, (_, i) => ({ get characterAttributes() { reads++; return { textFont: { name: fontAt(i), family: "F", style: "R" } }; } }));
  const ca = { textFont: { name: fontAt(0), family: "F", style: "R" } };
  return { para: { characterAttributes: ca, characters: chars, contents: textAt ? textAt(n) : "x".repeat(n) }, reads: () => reads };
};

test("đoạn >200 ký tự trả font đầu tiên, font khác ở cuối → thấy cả hai, đọc có trần", () => {
  const b = bigPara(1000, (i) => (i >= 990 ? "DMSerifDisplay-Regular" : "Chonburi-Regular"));
  const r = scanJson(makeHost({ docs: [doc([frame2027([], [b.para])])], installed: ["ArialMT"] }));
  assert.deepEqual(r.docs[0].fonts.map((f) => f.ps).sort(), ["Chonburi-Regular", "DMSerifDisplay-Regular"]);
  assert.ok(b.reads() <= 260, "số lần đọc: " + b.reads());
});

test("đoạn đồng nhất >200 ký tự: chỉ ~16 mẫu thưa, viaChars không tăng", () => {
  const b = bigPara(5000, () => "Chonburi-Regular");
  const r = scanJson(makeHost({ docs: [doc([frame2027([], [b.para])])], installed: ["ArialMT"] }));
  assert.deepEqual(r.docs[0].fonts.map((f) => f.ps), ["Chonburi-Regular"]);
  assert.ok(b.reads() <= 20, "số lần đọc: " + b.reads());
  assert.ok(!r.stats.viaChars);
});

test("ký tự trắng khác font (MyriadPro) bị bỏ qua khi lấy mẫu", () => {
  const text = "ab cd ef gh ij kl";
  const b = bigPara(text.length, (i) => (text.charAt(i) === " " ? "MyriadPro-Regular" : "Chonburi-Regular"), () => text);
  const r = scanJson(makeHost({ docs: [doc([frame2027([], [b.para])])], installed: ["ArialMT"] }));
  assert.deepEqual(r.docs[0].fonts.map((f) => f.ps), ["Chonburi-Regular"]);
});

test("đoạn 100 ký tự: font khác ở GIỮA (5 ký tự) vẫn thấy dù đầu/cuối cùng font", () => {
  const b = bigPara(100, (i) => (i >= 47 && i < 52 ? "DMSerifDisplay-Regular" : "Chonburi-Regular"));
  const r = scanJson(makeHost({ docs: [doc([frame2027([], [b.para])])], installed: ["ArialMT"] }));
  assert.deepEqual(r.docs[0].fonts.map((f) => f.ps).sort(), ["Chonburi-Regular", "DMSerifDisplay-Regular"]);
  assert.ok(b.reads() <= 205, "số lần đọc: " + b.reads());
});

test("ký tự đầu là khoảng trắng: font 'của vùng' (font ký tự đầu) không được tính", () => {
  const text = " abcdefghij";
  const b = bigPara(text.length, (i) => (i === 0 ? "MyriadPro-Regular" : "Chonburi-Regular"), () => text);
  const r = scanJson(makeHost({ docs: [doc([frame2027([], [b.para])])], installed: ["ArialMT"] }));
  assert.deepEqual(r.docs[0].fonts.map((f) => f.ps), ["Chonburi-Regular"]);
});

test("bộ đếm thử lại được reset khi có phản hồi trọn vẹn: 5 lần bận rồi thành công, sau đó tài liệu đổi vẫn được thử lại đủ 6 lần", async () => {
  const frames = [frameOf(run("a", "Chonburi-Regular"))];
  const host = makeHost({ docs: [doc(frames)] });
  const busy = (b) => ({ status: 200, json: { ok: true, results: b.fonts.map((f) => ({ name: f.ps, status: "busy" })), found: [], missing: [] } });
  const ok = (b) => ({ status: 200, json: { ok: true, results: b.fonts.map((f) => ({ name: f.ps, status: "already_active" })), found: b.fonts.map((f) => f.ps), missing: [] } });
  const p = makePanel(host, { respond: (b, n) => (n === 6 ? ok(b) : busy(b)) });
  await p.advance(40000);
  assert.equal(p.posts.length, 6, "5 lần bận + 1 lần thành công");
  frames.push(frameOf(run("b", "DMSerifDisplay-Regular")));           // tài liệu đổi (số frame đổi → chữ ký đổi)
  await p.advance(120000);
  assert.equal(p.posts.length, 6 + 1 + 6, "sau reset: 1 lần gửi + 6 lần thử lại (không reset thì chỉ còn 1 lần thử lại)");
});
