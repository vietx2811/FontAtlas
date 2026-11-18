#!/usr/bin/env python3
"""Dựng dữ liệu thử cho tự kiểm «cloud tải xong» (chỉ font fixture của repo, tên đã đổi bằng patch_font_names.py).
python3 prep.py <Q>   (Q: thư mục thử trên ổ dữ liệu). Cần chạy mkpool.py trước:
  python3 mkpool.py UX/src-tauri/crates/fontatlas-core/tests/fixtures UX/src-tauri/tests/qa/patch_font_names.py <Q>/base 1500 b
  python3 mkpool.py … <Q>/pool-cloud 61 c
Sau đó: base chia 30 thư mục con (Family00..29 × 50 file); Dupes = 12 bản cùng tên PostScript nhưng khác nội dung (thêm 2 byte) để thanh bên có mục Duplicates;
cloud = 31 stub `.TÊN.ttf.icloud` (kịch bản A, B) + 30 file 0 byte (kịch bản C)."""
import os, shutil, sys
Q = sys.argv[1]; base = Q + '/base'; fl = sorted(f for f in os.listdir(base) if f.endswith('.ttf'))
for i, f in enumerate(fl):
    d = os.path.join(base, 'Family%02d' % (i // 50)); os.makedirs(d, exist_ok=True); shutil.move(os.path.join(base, f), os.path.join(d, f))
os.makedirs(base + '/Dupes', exist_ok=True)
for f in fl[:12]:
    p = os.path.join(base, 'Dupes', f); shutil.copyfile(os.path.join(base, 'Family00', f), p); open(p, 'ab').write(b'QA')
os.makedirs(Q + '/small100', exist_ok=True)
for d in ('Family00', 'Family01'):
    for f in os.listdir(os.path.join(base, d)): shutil.copyfile(os.path.join(base, d, f), os.path.join(Q, 'small100', f))
