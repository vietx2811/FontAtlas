#!/usr/bin/env python3
"""Lọc đầu ra cargo test: thêm mốc thời gian (giây từ lúc bắt đầu) trước các dòng Compiling/Finished/Running/Doc-tests/"finished in"
và các dòng "has been running for over", để biết thời gian đi đâu. Mã thoát là của pipeline gọi (dùng pipefail)."""
import re, sys, time
t0 = time.time()
mark = re.compile(r"^\s*(Compiling|Finished|Running|Doc-tests|Blocking)\b|finished in|has been running for over")
for line in sys.stdin:
    if mark.search(line):
        line = "[+%5ds] %s" % (time.time() - t0, line)
    sys.stdout.write(line); sys.stdout.flush()
