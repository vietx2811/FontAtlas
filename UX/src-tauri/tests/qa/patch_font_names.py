#!/usr/bin/env python3
"""Đổi tên (bảng `name`) của BẢN SAO font fixture trong thư mục QA tạm, để QA không bao giờ va chạm với font thật đã cài/đăng ký trên máy
(RightFont, Adobe Fonts, Font Book…). Quy tắc xác định (cùng đầu vào ⇒ cùng byte), chỉ thư viện chuẩn Python:

  family  (ID 1, 16, 21)  "Roboto"            → "Roboto FAQA"
  full    (ID 4, 18)      "Roboto Regular"    → "Roboto FAQA Regular"
  unique  (ID 3)          "…"                 → "…;FAQA"
  PostScript (ID 6)       "Roboto-Regular"    → "RobotoFAQA-Regular"
  VF PS prefix (ID 25) và tên PS của instance (ID ≥ 256 dạng "<PSgốc>-…") được đổi cùng quy tắc.

  python3 patch_font_names.py [--suffix FAQA] <thư mục|file.ttf> …   # sửa tại chỗ; in "<file>\\t<PS cũ>\\t<PS mới>"
Hai bản đã có cùng PostScript gốc (VF và static của cùng họ) vẫn cùng PostScript sau khi đổi ⇒ các bước QA về xung đột/«Giữ cả hai» còn nguyên nghĩa.
Không đụng bảng nào khác ngoài `name` và `head.checkSumAdjustment`. KHÔNG dùng cho font ngoài thư mục QA tạm.
"""
import os
import struct
import sys

SUFFIX = "FAQA"


def checksum(data):
    data += b"\0" * (-len(data) % 4)
    return sum(struct.unpack(">%dI" % (len(data) // 4), data)) & 0xFFFFFFFF


def decode(pid, raw):
    return raw.decode("utf-16-be" if pid in (0, 3) else "mac_roman", "replace")


def encode(pid, text):
    return text.encode("utf-16-be" if pid in (0, 3) else "mac_roman", "replace")


def patch_name_table(nt, suffix):
    fmt, count, sto = struct.unpack(">HHH", nt[:6])
    recs = []
    for i in range(count):
        pid, eid, lid, nid, ln, off = struct.unpack(">6H", nt[6 + 12 * i: 18 + 12 * i])
        recs.append([pid, eid, lid, nid, nt[sto + off: sto + off + ln]])
    tail = b""
    if fmt == 1:  # langTag records: giữ nguyên
        p = 6 + 12 * count
        (n_lang,) = struct.unpack(">H", nt[p: p + 2])
        tail_recs = nt[p: p + 2 + 4 * n_lang]
        lang_strings = []
        for j in range(n_lang):
            ln, off = struct.unpack(">2H", nt[p + 2 + 4 * j: p + 6 + 4 * j])
            lang_strings.append(nt[sto + off: sto + off + ln])
        tail = (tail_recs, lang_strings)
    by = {(r[0], r[2], r[3]): decode(r[0], r[4]) for r in recs}

    def orig_ps(pid, lid):
        v = by.get((pid, lid, 6)) or next((t for (p, l, n), t in by.items() if n == 6), None)
        return v

    ps_any = next((t for (p, l, n), t in by.items() if n == 6), "")
    ps_base = ps_any.split("-")[0] if ps_any else ""
    old_ps, new_ps = ps_any, None
    for r in recs:
        pid, eid, lid, nid, raw = r
        t = decode(pid, raw)
        if nid in (1, 16, 21):
            n = t + " " + suffix
        elif nid in (4, 18):
            fam = by.get((pid, lid, 16)) or by.get((pid, lid, 1)) or ""
            n = fam + " " + suffix + t[len(fam):] if fam and t.startswith(fam) else t + " " + suffix
        elif nid == 3:
            n = t + ";" + suffix
        elif nid == 6:
            base, sep, rest = t.partition("-")
            n = base + suffix + sep + rest
            new_ps = n
        elif nid == 25:
            n = t + suffix
        elif nid >= 256 and ps_base and t.startswith(ps_base + "-"):
            n = ps_base + suffix + t[len(ps_base):]
        else:
            continue
        r[4] = encode(pid, n)
    recs.sort(key=lambda r: (r[0], r[1], r[2], r[3]))
    storage = b""
    out_recs = []
    for pid, eid, lid, nid, raw in recs:
        out_recs.append(struct.pack(">6H", pid, eid, lid, nid, len(raw), len(storage)))
        storage += raw
    if fmt == 1:
        tail_recs, lang_strings = tail
        head_tail = struct.pack(">H", len(lang_strings))
        for s in lang_strings:
            head_tail += struct.pack(">2H", len(s), len(storage))
            storage += s
    else:
        head_tail = b""
    sto_new = 6 + 12 * len(recs) + len(head_tail)
    nt2 = struct.pack(">HHH", fmt, len(recs), sto_new) + b"".join(out_recs) + head_tail + storage
    return nt2, old_ps, new_ps


def patch_file(path, suffix):
    d = open(path, "rb").read()
    tag = d[:4]
    if tag not in (b"\x00\x01\x00\x00", b"OTTO", b"true"):
        raise SystemExit("%s: không phải sfnt đơn (TTC/woff không được hỗ trợ)" % path)
    n = struct.unpack(">H", d[4:6])[0]
    tabs = []
    for i in range(n):
        t, cs, off, ln = struct.unpack(">4sIII", d[12 + 16 * i: 28 + 16 * i])
        tabs.append([t, cs, off, ln])
    blobs = {}
    for t, cs, off, ln in tabs:
        blobs[t] = d[off: off + ln]
    if b"name" not in blobs:
        raise SystemExit("%s: không có bảng name" % path)
    blobs[b"name"], old_ps, new_ps = patch_name_table(blobs[b"name"], suffix)
    # head.checkSumAdjustment tính lại sau khi ghi
    out = bytearray(d[:12 + 16 * n])
    pos = len(out)
    order = sorted(range(n), key=lambda i: tabs[i][2])  # giữ thứ tự dữ liệu gốc
    new_off = {}
    for i in order:
        t = tabs[i][0]
        b = blobs[t]
        if t == b"head" and len(b) >= 12:
            b = b[:8] + b"\0\0\0\0" + b[12:]
        pad = (-pos) % 4
        out += b"\0" * pad; pos += pad
        new_off[i] = pos
        out += b; pos += len(b)
        blobs[t] = b
    out += b"\0" * ((-pos) % 4)
    for i in range(n):
        t = tabs[i][0]; b = blobs[t]
        out[12 + 16 * i: 28 + 16 * i] = struct.pack(">4sIII", t, checksum(b), new_off[i], len(b))
    adj = (0xB1B0AFBA - checksum(bytes(out))) & 0xFFFFFFFF
    for i in range(n):
        if tabs[i][0] == b"head":
            o = new_off[i] + 8
            out[o: o + 4] = struct.pack(">I", adj)
    open(path, "wb").write(bytes(out))
    return old_ps, new_ps


def main(argv):
    suffix = SUFFIX
    if argv[:1] == ["--suffix"]:
        suffix, argv = argv[1], argv[2:]
    files = []
    for a in argv:
        if os.path.isdir(a):
            files += [os.path.join(a, f) for f in sorted(os.listdir(a)) if f.lower().endswith((".ttf", ".otf"))]
        else:
            files.append(a)
    for f in files:
        old, new = patch_file(f, suffix)
        print("%s\t%s\t%s" % (os.path.basename(f), old, new))


if __name__ == "__main__":
    main(sys.argv[1:])
