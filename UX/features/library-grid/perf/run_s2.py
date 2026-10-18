"""Harness S2 (spec 1.5): lưới 5.000 font trên WKWebView — FPS, p95 khung, % ô trống, bộ nhớ WebContent.

Chạy: npm run perf:s2 [-- --mode mask|img] [--n 5000] [--runs 3]   (macOS, cần Xcode CLT, Rust, Node)
Exit 0 nếu trung vị đạt ngưỡng S2, 1 nếu không. Ghi perf/.data/runs.jsonl.
"""
import argparse
import json
import os
import statistics
import subprocess
import sys
import time
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
UX = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
TAURI = os.path.join(UX, "src-tauri")
DATA = os.path.join(HERE, ".data")
PREVIEWS = os.path.join(DATA, "previews")
PORT = 5199
S2 = {"fps": (">=", 58.0), "frame_p95": ("<=", 20.0), "slow_blank_pct": ("==", 0.0), "proc_mb": ("<", 400.0)}
OPS = {">=": lambda a, b: a >= b, "<=": lambda a, b: a <= b, "==": lambda a, b: a == b, "<": lambda a, b: a < b}


def sh(cmd, **kw):
    print("$", " ".join(cmd), flush=True)
    subprocess.run(cmd, check=True, **kw)


def machine():
    cpu = subprocess.run(["sysctl", "-n", "machdep.cpu.brand_string"], capture_output=True, text=True).stdout.strip()
    mem = int(subprocess.run(["sysctl", "-n", "hw.memsize"], capture_output=True, text=True).stdout.strip() or 0) >> 30
    osv = subprocess.run(["sw_vers", "-productVersion"], capture_output=True, text=True).stdout.strip()
    return f"{cpu}, {mem} GB, macOS {osv}"


def run_wk(url):
    p = subprocess.run([os.path.join(HERE, "wk"), url], capture_output=True, text=True, timeout=300)
    lines = p.stdout.strip().splitlines()
    if not lines:
        raise RuntimeError(f"wk không trả kết quả: {p.stderr[-300:]}")
    r = json.loads(lines[0])
    for line in lines:
        if "phys_footprint:" in line:
            parts = line.split("phys_footprint:")[1].split()
            val = float(parts[0])
            r["proc_mb"] = val * 1024 if parts[1].upper().startswith("G") else val
    return r


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--mode", choices=["mask", "img"], default="mask")
    ap.add_argument("--n", type=int, default=5000)
    ap.add_argument("--runs", type=int, default=3)
    a = ap.parse_args()
    os.makedirs(DATA, exist_ok=True)

    if not os.path.exists(os.path.join(PREVIEWS, "manifest.json")):
        sh(["cargo", "run", "--release", "-p", "fontatlas-preview", "--example", "dump_previews", "--",
            "--out", PREVIEWS, "/System/Library/Fonts", "/Library/Fonts"], cwd=TAURI)
    else:
        sh(["cargo", "build", "--release", "-p", "fontatlas-preview", "--example", "dump_previews"], cwd=TAURI)
    sh(["swiftc", "-O", os.path.join(HERE, "wk.swift"), "-o", os.path.join(HERE, "wk")])

    vite = subprocess.Popen(["npx", "vite", "--config", os.path.join(HERE, "vite.config.ts"), "--port", str(PORT), "--strictPort"],
                            cwd=UX, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        t0 = time.time()
        while True:
            try:
                urllib.request.urlopen(f"http://localhost:{PORT}/api/manifest", timeout=2)
                break
            except Exception:
                if time.time() - t0 > 30:
                    raise RuntimeError("Vite không lên trong 30 s")
                time.sleep(0.5)
        runs = []
        for k in range(a.runs):
            r = run_wk(f"http://localhost:{PORT}/?n={a.n}&mode={a.mode}&auto=1")
            r["rep"] = k
            runs.append(r)
            with open(os.path.join(DATA, "runs.jsonl"), "a") as f:
                f.write(json.dumps(r) + "\n")
            print(json.dumps({x: r.get(x) for x in ["mode", "rep", "first_ms", "fps", "frame_p95", "blank_pct", "slow_blank_pct", "text_change_ms", "fontfaces", "proc_mb"]}), flush=True)
    finally:
        vite.terminate()

    med = {k: statistics.median(r[k] for r in runs) for k in ["first_ms", "fps", "frame_p95", "blank_pct", "slow_fps", "slow_blank_pct", "text_change_ms", "fontfaces", "dom", "proc_mb"]}
    print(f"\nMáy: {machine()} · mode={a.mode} · n={a.n} · {a.runs} lần (trung vị)")
    ok = True
    for k, v in med.items():
        rule = S2.get(k)
        good = OPS[rule[0]](v, rule[1]) if rule else None
        ok &= good is not False
        print(f"  {k:16} {v:10.1f}  {'' if rule is None else ('ĐẠT' if good else 'TRƯỢT') + f' (cần {rule[0]} {rule[1]})'}")
    if not ok and a.mode == "mask":
        print("\nS2 trượt với mask → đo phương án dự phòng: npm run perf:s2 -- --mode img")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
