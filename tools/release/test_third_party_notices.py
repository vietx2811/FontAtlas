#!/usr/bin/env python3
"""Kiểm logic danh sách giấy phép của third_party_notices.py (không cần cargo/npm): python3 tools/release/test_third_party_notices.py"""
import os, sys
sys.path.insert(0, os.path.dirname(__file__))
import third_party_notices as t

ok = [("MIT", True), ("MIT OR Apache-2.0", True), ("MIT/Apache-2.0", True), ("Apache-2.0 WITH LLVM-exception", True),
      ("(MIT OR Apache-2.0) AND Unicode-3.0", True), ("MIT OR GPL-3.0-only", True),
      ("GPL-3.0-only", False), ("GPL-2.0-or-later", False), ("MIT AND GPL-2.0-only", False), ("LGPL-2.1-only", False),
      ("AGPL-3.0", False), ("SSPL-1.0", False), ("CC-BY-NC-4.0", False), ("", False)]
bad = [(e, w) for e, w in ok if (t.spdx_ok(e) if e else False) != w]
for e, w in ok:
    print(("ok  " if (t.spdx_ok(e) if e else False) == w else "FAIL"), repr(e), "->", w)
# build(): gói GPL phải bị bắt, gói không khai báo phải bị bắt
real_rust, real_npm = t.rust_packages, t.npm_packages
t.rust_packages = lambda: [{"eco": "rust", "name": "gplcrate", "version": "1", "license": "GPL-3.0-only", "license_file": None, "repo": "", "dir": "/nonexistent"},
                           {"eco": "rust", "name": "nolic", "version": "1", "license": None, "license_file": None, "repo": "", "dir": "/nonexistent"}]
t.npm_packages = lambda: []
_, found = t.build()
names = sorted(p["name"] for p, _ in found)
print(("ok  " if names == ["gplcrate", "nolic"] else "FAIL"), "build() bắt gói GPL và gói không khai báo:", names)
sys.exit(1 if bad or names != ["gplcrate", "nolic"] else 0)
