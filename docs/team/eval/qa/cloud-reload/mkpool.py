#!/usr/bin/env python3
"""Tạo kho font thử cho tự kiểm «cloud tải xong»: N biến thể của fixture repo, mỗi bản có tên PostScript/family duy nhất (hậu tố FAQ####).
python3 mkpool.py <fixtures> <patch_font_names.py> <thư mục đích> <N> <tiền tố tên file>"""
import importlib.util, os, shutil, sys
fx, pf, dst, n, pre = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4]), sys.argv[5]
spec = importlib.util.spec_from_file_location("pf", pf); m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
files = sorted(f for f in os.listdir(fx) if f.endswith(".ttf") and not f.startswith("roboto_vf"))
os.makedirs(dst, exist_ok=True)
for i in range(n):
    f = files[i % len(files)]
    out = os.path.join(dst, "%s%04d_%s" % (pre, i, f))
    shutil.copyfile(os.path.join(fx, f), out)
    m.patch_file(out, "FAQ%s%04d" % (pre.upper(), i))
print(dst, n)
