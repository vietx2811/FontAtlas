#!/usr/bin/env python3
"""Tạo THIRD-PARTY-NOTICES cho bản phát hành (Rust + npm), và chặn giấy phép ngoài danh sách cho phép.

  python3 tools/release/third_party_notices.py            # kiểm giấy phép rồi ghi UX/src-tauri/resources/THIRD-PARTY-NOTICES.txt
  python3 tools/release/third_party_notices.py --check    # kiểm giấy phép + file đã commit còn khớp (không ghi), thoát 1 nếu lệch/vi phạm

Nguồn dữ liệu (không tải gì, không cần cargo-about / license-checker):
  * Rust: `cargo metadata --locked --offline` cho target phát hành; chỉ các crate đi vào app (cạnh phụ thuộc thường,
    bỏ dev/build-only), trừ crate của chính dự án. Văn bản giấy phép đọc từ thư mục nguồn crate trong ~/.cargo.
  * npm: `dependencies` của UX/package.json và toàn bộ phụ thuộc bắc cầu của chúng, đọc từ UX/node_modules (cần `npm ci`).
Giấy phép phải thuộc ALLOWED (biểu thức SPDX: `A OR B` chỉ cần một vế được phép; `A AND B` cần cả hai).
Gói không khai báo giấy phép: phải có trong OVERRIDES kèm lý do, nếu không bước thất bại.
Kết quả có thứ tự cố định và không có thời gian, nên --check so sánh từng byte.
"""
import hashlib
import json
import os
import re
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
TAURI = os.path.join(ROOT, "UX", "src-tauri")
UX = os.path.join(ROOT, "UX")
OUT = os.path.join(TAURI, "resources", "THIRD-PARTY-NOTICES.txt")
TARGET = "aarch64-apple-darwin"

# Danh sách giấy phép được phép (định danh SPDX). Thêm một dòng ở đây là một quyết định pháp lý: ghi lý do trong commit.
ALLOWED = {
    "MIT", "MIT-0", "Apache-2.0", "Apache-2.0 WITH LLVM-exception", "BSD-2-Clause", "BSD-3-Clause", "ISC", "Zlib",
    "Unicode-3.0", "Unicode-DFS-2016", "CC0-1.0", "0BSD", "BSL-1.0", "Unlicense", "CDLA-Permissive-2.0",
    "MPL-2.0",   # copyleft cấp tệp (chỉ buộc công khai sửa đổi chính các tệp MPL); đang có 5 crate (cssparser, selectors, option-ext…) — chủ app cần xác nhận
    "BlueOak-1.0.0", "Python-2.0",
}
# Gói không có trường giấy phép dùng được: (tên, phiên bản hoặc "*") -> (SPDX, lý do).
OVERRIDES = {}
# Crate của chính dự án (không liệt kê).
OWN_PREFIX = ("fontatlas-",)
OWN_NAMES = {"app"}


# ---------- Dữ liệu và mô hình đi kèm app ----------
# Mỗi mục: tên, tệp (đường dẫn từ gốc repo; có thể dùng *), nguồn/giấy phép, ghi chú, và các tệp văn bản giấy phép/thông báo
# được chép nguyên văn vào THIRD-PARTY-NOTICES. Cổng chặn: mọi tệp dữ liệu được commit trong UX/public/, crates/*/data/ và
# resources/ phải thuộc một mục ở đây; thêm tệp dữ liệu mới mà chưa kiểm giấy phép là bước phát hành thất bại.
DATA_ITEMS = [
    {"name": "Inter, JetBrains Mono, Newsreader (UI fonts, woff2 in the web bundle)",
     "files": ["UX/public/fonts/*.woff2", "UX/public/fonts/OFL-*.txt"],
     "license": "SIL Open Font License 1.1", "origin": "The Inter, JetBrains Mono and Newsreader project authors (see the copyright lines below)",
     "note": "Variable-font woff2 subsets (latin, latin-ext, vietnamese) embedded in the app's frontend; licence texts follow.",
     "texts": ["UX/public/fonts/OFL-Inter.txt", "UX/public/fonts/OFL-JetBrainsMono.txt", "UX/public/fonts/OFL-Newsreader.txt"]},
    {"name": "SigLIP 2 image encoder (vision tower), downloaded at first use",
     "files": ["UX/src-tauri/crates/fontatlas-embed/data/model-manifest.json", "UX/src-tauri/crates/fontatlas-embed/data/model-NOTICE.txt",
               "UX/src-tauri/crates/fontatlas-embed/data/model-LICENSE-Apache-2.0.txt"],
     "license": "Apache License 2.0", "origin": "google/siglip2-base-patch16-224 (Google), Hugging Face, revision 75de2d55ec2d0b4efc50b3e9ad70dba96a7b2fa2",
     "note": "Not shipped inside the app: Font Atlas downloads the weights directly from Hugging Face on the user's machine and keeps only the vision tensors, unchanged (details in the notice below). The manifest in the app pins the revision and checksums.",
     "texts": ["UX/src-tauri/crates/fontatlas-embed/data/model-NOTICE.txt", "UX/src-tauri/crates/fontatlas-embed/data/model-LICENSE-Apache-2.0.txt"]},
    {"name": "Language data (Hyperglot)",
     "files": ["UX/src-tauri/crates/fontatlas-core/data/langdata.json", "UX/src-tauri/resources/NOTICE-hyperglot.md", "UX/src-tauri/resources/LICENSE-hyperglot.txt"],
     "license": "Apache License 2.0", "origin": "Hyperglot 0.8.2 by Rosetta Type/Research and contributors",
     "note": "langdata.json is converted from the Hyperglot data files; the full licence is bundled as LICENSE-hyperglot.txt next to this file.",
     "texts": ["UX/src-tauri/resources/NOTICE-hyperglot.md"]},
    {"name": "Whitening matrix (whitening-v2.bin) and reference quantiles (reference-v1.json)",
     "files": ["UX/src-tauri/crates/fontatlas-embed/data/whitening-v2.bin", "UX/src-tauri/crates/fontatlas-index/data/reference-v1.json"],
     "license": "Font Atlas's own data (derived statistics)", "origin": "Computed by Font Atlas from the open font collection google/fonts (revision 47d54ca1ebb14fd864695f1d39dc734f39017c33, OFL and Apache-2.0 fonts): the whitening matrix is fitted on the 1,729 faces of the \"tune\" half; the reference quantiles cover 3,813 faces",
     "note": "Numeric matrices and percentile tables only; they contain no font outlines or font files.",
     "texts": []},
    {"name": "Hand-made tables and text written for Font Atlas",
     "files": ["UX/src-tauri/crates/fontatlas-describe/data/descriptor-dictionary.json", "UX/src-tauri/crates/fontatlas-index/data/superfamilies-v1.csv",
               "UX/src-tauri/resources/en.lproj/*", "UX/src-tauri/resources/vi.lproj/*"],
     "license": "Font Atlas's own work", "origin": "Font Atlas project",
     "note": "Descriptor dictionary, superfamily table (family names), and localisation strings.",
     "texts": []},
]
DATA_GLOBS = ["UX/public/*", "UX/src-tauri/crates/*/data/*", "UX/src-tauri/resources/*"]
DATA_IGNORE = {"UX/src-tauri/resources/THIRD-PARTY-NOTICES.txt"}


def data_files_unlisted():
    import fnmatch
    out = subprocess.run(["git", "ls-files", "--"] + DATA_GLOBS, capture_output=True, text=True, cwd=ROOT)
    tracked = [f for f in out.stdout.splitlines() if f not in DATA_IGNORE and "/tests/" not in f]  # fixtures của test không đi vào app
    pats = [x for it in DATA_ITEMS for x in it["files"]]
    return [f for f in tracked if not any(fnmatch.fnmatch(f, pt) for pt in pats)]


def data_missing():
    miss = []
    for it in DATA_ITEMS:
        for f in it["texts"] + [x for x in it["files"] if "*" not in x]:
            if not os.path.isfile(os.path.join(ROOT, f)):
                miss.append(f)
    return miss


def render_data():
    L = ["2. DATA AND MODELS", "-" * 72]
    for it in DATA_ITEMS:
        L.append("")
        L.append("* " + it["name"])
        L.append("  Licence: " + it["license"])
        L.append("  Origin:  " + it["origin"])
        L.append("  " + it["note"])
        for t in it["texts"]:
            L.append("")
            L.append("  --- " + os.path.basename(t) + " ---")
            L.extend("  " + ln if ln.strip() else "" for ln in read(os.path.join(ROOT, t)).split("\n"))
    L.append("")
    return L


def die(msg):
    print("THIRD-PARTY-NOTICES: " + msg, file=sys.stderr)
    sys.exit(1)


# ---------- SPDX ----------
def spdx_ok(expr):
    """True nếu biểu thức SPDX thoả danh sách ALLOWED. Hỗ trợ OR, AND, WITH, ngoặc, và '/' cũ (= OR)."""
    expr = expr.replace("/", " OR ")
    toks = re.findall(r"\(|\)|[^\s()]+", expr)
    pos = 0

    def peek():
        return toks[pos] if pos < len(toks) else None

    def take():
        nonlocal pos
        pos += 1
        return toks[pos - 1]

    def atom():
        t = take()
        if t == "(":
            v = parse_or()
            if peek() == ")":
                take()
            return v
        name = t.rstrip("+")
        if peek() == "WITH":
            take()
            name = name + " WITH " + take()
        return name in ALLOWED or t in ALLOWED

    def parse_and():
        v = atom()
        while peek() == "AND":
            take()
            v = atom() and v
        return v

    def parse_or():
        v = parse_and()
        while peek() == "OR":
            take()
            v = parse_and() or v
        return v

    return parse_or()


# ---------- giấy phép của thư mục gói ----------
LIC_RE = re.compile(r"^(LICEN[CS]E|COPYING|NOTICE|UNLICENSE)([-_.].*)?$", re.I)


def license_files(d):
    out = []
    if not os.path.isdir(d):
        return out
    for n in sorted(os.listdir(d)):
        p = os.path.join(d, n)
        if os.path.isfile(p) and LIC_RE.match(n) and not n.lower().endswith((".rs", ".js", ".json")):
            out.append(p)
    return out


def read(p):
    with open(p, "rb") as f:
        return f.read().decode("utf-8", "replace").replace("\r\n", "\n").strip()


# ---------- Rust ----------
def rust_packages():
    env = dict(os.environ)
    r = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--offline", "--filter-platform", TARGET,
         "--manifest-path", os.path.join(TAURI, "Cargo.toml")],
        capture_output=True, text=True, env=env)
    if r.returncode != 0:
        die("cargo metadata lỗi (cần đã `cargo fetch`/build trước):\n" + r.stderr[-800:])
    md = json.loads(r.stdout)
    pk = {p["id"]: p for p in md["packages"]}
    nodes = {n["id"]: n for n in md["resolve"]["nodes"]}
    seen, stack = set(), [md["resolve"]["root"] or next(iter(md["workspace_members"]))]
    members = set(md["workspace_members"])
    # Gốc: mọi thành viên workspace phát hành (app); đi theo cạnh thường (kind None).
    stack = [m for m in md["workspace_members"]]
    while stack:
        i = stack.pop()
        if i in seen:
            continue
        seen.add(i)
        for d in nodes[i]["deps"]:
            if any(k.get("kind") is None for k in d["dep_kinds"]):
                stack.append(d["pkg"])
    res = []
    for i in seen:
        p = pk[i]
        if i in members or p["name"] in OWN_NAMES or p["name"].startswith(OWN_PREFIX):
            continue
        res.append({
            "eco": "rust", "name": p["name"], "version": p["version"], "license": p.get("license"),
            "license_file": p.get("license_file"), "repo": p.get("repository") or p.get("homepage") or "",
            "dir": os.path.dirname(p["manifest_path"]),
        })
    return res


# ---------- npm ----------
def npm_packages():
    nm = os.path.join(UX, "node_modules")
    if not os.path.isdir(nm):
        die("thiếu UX/node_modules: chạy `npm ci` trong UX/ trước")
    root = json.load(open(os.path.join(UX, "package.json")))

    def find(name, frm):
        d = frm
        while True:
            c = os.path.join(d, "node_modules", name)
            if os.path.isfile(os.path.join(c, "package.json")):
                return c
            if os.path.abspath(d) in (os.path.abspath(UX), "/", os.path.dirname(os.path.abspath(UX))):
                break
            d = os.path.dirname(d)
        c = os.path.join(nm, name)
        return c if os.path.isfile(os.path.join(c, "package.json")) else None

    seen, res, stack = {}, [], [(n, UX) for n in root.get("dependencies", {})]
    while stack:
        name, frm = stack.pop()
        d = find(name, frm)
        if d is None:
            continue  # phụ thuộc tuỳ chọn không cài trên máy này
        d = os.path.realpath(d)
        if d in seen:
            continue
        pj = json.load(open(os.path.join(d, "package.json")))
        seen[d] = 1
        lic = pj.get("license")
        if isinstance(lic, dict):
            lic = lic.get("type")
        if lic is None and isinstance(pj.get("licenses"), list):
            lic = " OR ".join(x.get("type", "?") for x in pj["licenses"])
        repo = pj.get("repository")
        repo = repo.get("url") if isinstance(repo, dict) else repo
        res.append({"eco": "npm", "name": pj.get("name", name), "version": pj.get("version", "?"), "license": lic,
                    "license_file": None, "repo": repo or pj.get("homepage") or "", "dir": d})
        for dep in list(pj.get("dependencies", {})) + list(pj.get("optionalDependencies", {})):
            stack.append((dep, d))
    return res


# ---------- tổng hợp ----------
def build():
    pkgs = rust_packages() + npm_packages()
    bad = []
    for p in pkgs:
        ov = OVERRIDES.get((p["name"], p["version"])) or OVERRIDES.get((p["name"], "*"))
        lic = ov[0] if ov else p["license"]
        p["spdx"] = lic
        if not lic:
            bad.append((p, "không khai báo giấy phép"))
        elif not spdx_ok(lic):
            bad.append((p, "giấy phép ngoài danh sách cho phép: " + lic))
    return pkgs, bad


def render(pkgs):
    pkgs = sorted(pkgs, key=lambda p: (p["eco"], p["name"].lower(), p["version"]))
    groups = {}  # hash văn bản -> (text, [pkg])
    nofile = []
    for p in pkgs:
        files = []
        if p["license_file"]:
            lf = p["license_file"] if os.path.isabs(p["license_file"]) else os.path.join(p["dir"], p["license_file"])
            if os.path.isfile(lf):
                files.append(lf)
        files += [f for f in license_files(p["dir"]) if f not in files]
        if not files:
            nofile.append(p)
            continue
        text = "\n\n".join(read(f) for f in files)
        h = hashlib.sha256(text.encode()).hexdigest()
        groups.setdefault(h, (text, []))[1].append(p)
    L = []
    L.append("FONT ATLAS — THIRD-PARTY NOTICES")
    L.append("=" * 72)
    L.append("Font Atlas includes the open-source components listed below. Each is the property of its authors and")
    L.append("is used under the licence shown. Section 2 covers the data, fonts and models that are not code; the")
    L.append("Hyperglot notice and licence are also bundled as NOTICE-hyperglot.md and LICENSE-hyperglot.txt in the same folder.")
    L.append("")
    L.append("Generated by tools/release/third_party_notices.py from Cargo.lock and UX/node_modules; do not edit by hand.")
    L.append("")
    L.append("1. COMPONENTS (%d)" % len(pkgs))
    L.append("-" * 72)
    for p in pkgs:
        L.append("%s %s %s — %s%s" % ("[rust]" if p["eco"] == "rust" else "[npm] ", p["name"], p["version"], p["spdx"],
                                        (" — " + p["repo"]) if p["repo"] else ""))
    L.append("")
    L.extend(render_data())
    L.append("3. LICENCE TEXTS OF CODE COMPONENTS")
    L.append("-" * 72)
    for h, (text, ps) in sorted(groups.items(), key=lambda kv: (-len(kv[1][1]), kv[0])):
        L.append("")
        L.append("### Used by %d component(s):" % len(ps))
        L.append(", ".join("%s %s" % (p["name"], p["version"]) for p in ps))
        L.append("")
        L.append(text)
        L.append("")
        L.append("-" * 72)
    if nofile:
        L.append("")
        L.append("### Components declaring a licence without a bundled licence file (the SPDX identifier above applies):")
        L.append(", ".join("%s %s (%s)" % (p["name"], p["version"], p["spdx"]) for p in nofile))
    return "\n".join(L).rstrip() + "\n"


def main():
    check = "--check" in sys.argv[1:]
    pkgs, bad = build()
    if bad:
        print("THIRD-PARTY-NOTICES: %d gói vi phạm danh sách giấy phép:" % len(bad), file=sys.stderr)
        for p, why in bad:
            print("  %s %s %s: %s" % (p["eco"], p["name"], p["version"], why), file=sys.stderr)
        print("Sửa: bỏ phụ thuộc đó, hoặc (quyết định pháp lý) thêm vào ALLOWED/OVERRIDES kèm lý do.", file=sys.stderr)
        sys.exit(1)
    miss, unl = data_missing(), data_files_unlisted()
    if miss or unl:
        for f in miss:
            print("THIRD-PARTY-NOTICES: thiếu tệp được liệt kê trong DATA_ITEMS: " + f, file=sys.stderr)
        for f in unl:
            print("THIRD-PARTY-NOTICES: tệp dữ liệu chưa được kiểm giấy phép (thêm vào DATA_ITEMS): " + f, file=sys.stderr)
        sys.exit(1)
    text = render(pkgs)
    n_rust = sum(1 for p in pkgs if p["eco"] == "rust")
    if check:
        cur = open(OUT, encoding="utf-8").read() if os.path.isfile(OUT) else None
        if cur != text:
            die("%s không khớp với Cargo.lock/node_modules hiện tại; chạy lại script không có --check rồi commit" % os.path.relpath(OUT, ROOT))
        print("ok: %d gói (%d rust, %d npm), giấy phép trong danh sách, file khớp" % (len(pkgs), n_rust, len(pkgs) - n_rust))
        return
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(text)
    print("đã ghi %s: %d gói (%d rust, %d npm), %d KB" % (os.path.relpath(OUT, ROOT), len(pkgs), n_rust, len(pkgs) - n_rust, len(text) // 1024))


if __name__ == "__main__":
    main()
