#!/usr/bin/env python3
"""Nghiên cứu (KHÔNG thuộc app): trích bảng ngôn ngữ → bộ ký tự từ Hyperglot và gflanguages,
đo kích thước/độ phủ, rồi đối chiếu thử với cmap của vài font mẫu.

Chuẩn bị (thư mục tạm, ngoài repo):
  mkdir -p /tmp/fa-langdata/dl && cd /tmp/fa-langdata/dl
  python3 -m pip download --no-deps hyperglot gflanguages -d .
  unzip -qo hyperglot-*.whl -d ../hg && unzip -qo gflanguages-*.whl -d ../gf
  python3 -m venv /tmp/fa-langdata/venv && /tmp/fa-langdata/venv/bin/pip install pyyaml fonttools
Chạy:
  /tmp/fa-langdata/venv/bin/python -I lang_extract.py /tmp/fa-langdata fonts/samples/*.ttf
"""
import gzip
import json
import re
import sys
import unicodedata
from collections import Counter
from pathlib import Path

import yaml
from fontTools.ttLib import TTFont

ROOT = Path(sys.argv[1])
FONTS = [Path(p) for p in sys.argv[2:]]
HG = ROOT / "hg/hyperglot"
GF = ROOT / "gf/gflanguages/data/languages"


def chars(s: str) -> list[str]:
    """Chuỗi cách nhau bởi dấu cách → danh sách ký tự; tách tổ hợp nhiều code point thành từng code point."""
    out = []
    for tok in (s or "").split():
        tok = tok.replace("◌", "")
        out.extend(unicodedata.normalize("NFC", tok))
    return out


# ---------- Hyperglot ----------
default = yaml.safe_load((HG / "extra_data/default.yaml").read_text())


def resolve(s, lang_db):
    """Thay <default>, <eng>, <eng auxiliary>… bằng nội dung thật (đủ cho mục đích đo)."""
    def rep(m):
        parts = m.group(1).split()
        code = parts[0]
        attr = parts[1] if len(parts) > 1 and parts[1] in ("base", "auxiliary", "marks") else "base"
        if code == "default":
            return ""
        src = lang_db.get(code)
        if not src:
            return ""
        orths = src.get("orthographies") or []
        return " ".join(str(o.get(attr, "")) for o in orths[:1])
    for _ in range(3):
        s = re.sub(r"<([^>]+)>", rep, s or "")
    return s


hg_db = {p.stem: yaml.safe_load(p.read_text()) for p in sorted((HG / "data").glob("*.yaml"))}
hg_rows = []
for code, d in hg_db.items():
    if not d or d.get("status", "living") != "living" or "includes" in d:
        continue
    for o in d.get("orthographies") or []:
        if o.get("status", "primary") != "primary":
            continue
        base = set(chars(resolve(str(o.get("base", "")), hg_db)))
        marks = set(chars(resolve(str(o.get("marks", "")), hg_db)))
        if not base:
            continue
        hg_rows.append({"id": f"{code}_{o.get('script')}", "name": d.get("preferred_name") or d.get("name"),
                        "autonym": o.get("autonym"), "script": o.get("script"), "validity": d.get("validity", "todo"),
                        "speakers": d.get("speakers"), "base": base, "marks": marks})

# ---------- gflanguages ----------
gf_rows = []
for p in sorted(GF.glob("*.textproto")):
    t = p.read_text()
    m = re.search(r'exemplar_chars \{(.*?)\n\}', t, re.S)
    if not m:
        continue
    base = re.search(r'\bbase: "(.*)"', m.group(1))
    marks = re.search(r'\bmarks: "(.*)"', m.group(1))
    if not base:
        continue
    hist = re.search(r'historical: true', t)
    pop = re.search(r'population: (\d+)', t)
    gf_rows.append({"id": p.stem, "script": p.stem.split("_")[-1], "historical": bool(hist),
                    "population": int(pop.group(1)) if pop else 0,
                    "base": set(chars(base.group(1))),
                    "marks": set(chars(marks.group(1))) if marks else set()})


def compact(rows):
    """Định dạng nhúng đề xuất: id → chuỗi ký tự base (UTF-8), nén gzip."""
    data = {r["id"]: "".join(sorted(r["base"])) for r in rows}
    raw = json.dumps(data, ensure_ascii=False, separators=(",", ":")).encode()
    return len(raw), len(gzip.compress(raw, 9))


print("== Hyperglot", (ROOT / "dl").glob("hyperglot-*.whl").__next__().name)
print("  file YAML:", len(hg_db), "| orthography primary sống có base:", len(hg_rows))
print("  validity:", dict(Counter(r["validity"] for r in hg_rows)))
print("  script top:", Counter(r["script"] for r in hg_rows).most_common(8))
print("  JSON base (raw, gzip) bytes:", compact(hg_rows))
print("  verified+preliminary:", compact([r for r in hg_rows if r["validity"] in ("verified", "preliminary")]),
      sum(r["validity"] in ("verified", "preliminary") for r in hg_rows))

print("== gflanguages", (ROOT / "dl").glob("gflanguages-*.whl").__next__().name)
print("  textproto:", len(list(GF.glob('*.textproto'))), "| có exemplar base:", len(gf_rows),
      "| không historical:", sum(not r["historical"] for r in gf_rows))
print("  script top:", Counter(r["script"] for r in gf_rows).most_common(8))
print("  JSON base (raw, gzip) bytes:", compact(gf_rows))

vi_hg = next(r for r in hg_rows if r["id"] == "vie_Latin")
vi_gf = next(r for r in gf_rows if r["id"] == "vi_Latn")
print("== Tiếng Việt: Hyperglot base", len(vi_hg["base"]), "| gflanguages base", len(vi_gf["base"]),
      "| khác nhau:", "".join(sorted(vi_hg["base"] ^ vi_gf["base"])) or "(không)")

# ---------- đối chiếu cmap ----------
latn = [r for r in hg_rows if r["script"] == "Latin" and r["validity"] in ("verified", "preliminary")]
for f in FONTS:
    cmap = set(chr(c) for c in TTFont(f, lazy=True).getBestCmap())
    ok = [r for r in hg_rows if r["base"] <= cmap]
    miss_vi = sorted(vi_hg["base"] - cmap)
    print(f"== {f.name}: hỗ trợ {len(ok)}/{len(hg_rows)} ngôn ngữ (Hyperglot base); Latin tin cậy {sum(r['base'] <= cmap for r in latn)}/{len(latn)};"
          f" scripts {sorted(Counter(r['script'] for r in ok))[:6]}; thiếu VI {len(miss_vi)}: {''.join(miss_vi[:12])}")

# Kiểm tra nhanh nhiễu: orthography không phải Latin mà font Latin vẫn "đạt" → thường do base kế thừa rỗng/khác script.
if FONTS:
    cmap = set(chr(c) for c in TTFont(FONTS[0], lazy=True).getBestCmap())
    odd = [(r["id"], "".join(sorted(r["base"]))[:20]) for r in hg_rows if r["script"] != "Latin" and r["base"] <= cmap]
    print("== nghi nhiễu (non-Latin đạt trên", FONTS[0].name, "):", odd[:5])
