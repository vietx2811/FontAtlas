// SPIKE — mở URL trong WKWebView (engine Tauri dùng trên macOS), chờ window.__result, in kết quả + bộ nhớ tiến trình WebContent.
// swiftc -O wk.swift -o wk && ./wk 'http://localhost:5199/?mode=b&n=2000&auto=1&tag=wkwebview'
import Cocoa
import WebKit

final class D: NSObject, NSApplicationDelegate {
    var win: NSWindow!
    var wv: WKWebView!
    let started = Date()

    func applicationDidFinishLaunching(_ n: Notification) {
        let frame = NSRect(x: 40, y: 40, width: 1280, height: 800)
        let cfg = WKWebViewConfiguration()
        cfg.websiteDataStore = .nonPersistent() // không dùng HTTP cache giữa các lần chạy
        wv = WKWebView(frame: frame, configuration: cfg)
        // tránh WebKit hãm rAF khi cửa sổ bị che
        let sel = Selector(("_setWindowOcclusionDetectionEnabled:"))
        if wv.responds(to: sel) { wv.perform(sel, with: nil) }
        win = NSWindow(contentRect: frame, styleMask: [.titled], backing: .buffered, defer: false)
        win.level = .floating
        win.contentView = wv
        win.makeKeyAndOrderFront(nil)
        win.orderFrontRegardless()
        wv.load(URLRequest(url: URL(string: CommandLine.arguments[1])!))
        Timer.scheduledTimer(withTimeInterval: 1, repeats: true) { _ in self.poll() }
    }

    func poll() {
        if Date().timeIntervalSince(started) > 240 { print("{\"error\":\"timeout\"}"); exit(1) }
        wv.evaluateJavaScript("window.__result ? JSON.stringify(window.__result) : ''") { r, _ in
            guard let s = r as? String, !s.isEmpty else { return }
            let pid = (self.wv.value(forKey: "_webProcessIdentifier") as? NSNumber)?.int32Value ?? 0
            let p = Process(); let pipe = Pipe()
            p.executableURL = URL(fileURLWithPath: "/usr/bin/footprint")
            p.arguments = ["-p", String(pid)]
            p.standardOutput = pipe; p.standardError = pipe
            try? p.run(); p.waitUntilExit()
            let out = String(data: pipe.fileHandleForReading.readDataToEndOfFile(), encoding: .utf8) ?? ""
            let fp = out.split(separator: "\n").first { $0.contains("phys_footprint:") }.map(String.init) ?? out.prefix(300).description
            print(s)
            print("WEBCONTENT_PID \(pid) \(fp.trimmingCharacters(in: .whitespaces))")
            exit(0)
        }
    }
}

let app = NSApplication.shared
app.setActivationPolicy(.accessory) // không cướp focus của người dùng
let d = D()
app.delegate = d
app.run()
