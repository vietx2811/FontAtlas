#!/usr/bin/env python3
"""Mô phỏng file cloud «tải xong»: thay stub `.NAME.ttf.icloud` bằng font thật (đổi tên/tạo file mới), hoặc ghi đè file rỗng bằng font thật.
python3 cloudops.py <Q> <A|B|C> <log.jsonl>   A: 1 file; B: 30 file cách 1 s (stub→thật); C: 30 file cách 4 s (ghi đè file 0 byte)"""
import json, os, shutil, sys, time
Q, sc, lg = sys.argv[1], sys.argv[2], sys.argv[3]
pool = os.path.join(Q, "pool-cloud"); cd = os.path.join(Q, "cloud")
files = sorted(os.listdir(pool))
def ms(): return int(time.time() * 1000)
def rec(**k): open(lg, "a").write(json.dumps(dict(t=ms(), **k)) + "\n")
def swap_stub(f):
    stub = os.path.join(cd, "." + f + ".icloud"); dst = os.path.join(cd, f)
    shutil.copyfile(os.path.join(pool, f), dst + ".tmpq"); os.rename(dst + ".tmpq", dst)  # file thật xuất hiện (rename)
    os.remove(stub); rec(op="swap", file=f)
def fill_empty(f):
    shutil.copyfile(os.path.join(pool, f), os.path.join(cd, f)); rec(op="fill", file=f)
if sc == "A": rec(op="start", sc=sc); swap_stub(files[0])
elif sc == "B":
    rec(op="start", sc=sc)
    for f in files[1:31]: swap_stub(f); time.sleep(1.0)
elif sc == "C":
    rec(op="start", sc=sc)
    for f in files[31:61]: fill_empty(f); time.sleep(4.0)
elif sc == "D":  # ghi đè file ĐÃ có bằng đúng nội dung cũ (chỉ đổi mtime): đồng bộ lại / chạm file; không có font mới
    rec(op="start", sc=sc)
    for f in files[:10]:
        p = os.path.join(cd, f); b = open(p, "rb").read(); open(p, "wb").write(b); rec(op="rewrite", file=f); time.sleep(4.0)
rec(op="end", sc=sc)
