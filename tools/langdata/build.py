#!/usr/bin/env python3
"""Chuyển dữ liệu Hyperglot (YAML) thành một file gọn cho fontatlas-core.

Dùng:  python3 tools/langdata/build.py <thư-mục-hyperglot-đã-giải-nén> <sha-commit> [out.json]
Cần PyYAML. Chỉ ĐỌC dữ liệu YAML (lib/hyperglot/data + extra_data), không chạy code Hyperglot,
không đụng thư mục other/ (license từng nguồn riêng, chưa kiểm).

Nguồn: https://github.com/rosettatype/hyperglot (Apache-2.0). Xem NOTICE-hyperglot.md.
Lọc: validity ∈ {verified, preliminary}; orthography status = primary; kiểm chéo script.
Kết quả: {"hyperglot_sha", "hyperglot_version", "converter": N, "langs": [{id, bcp47, name_en,
autonym, script, base, marks}]}; base/marks là chuỗi các grapheme cách nhau bằng khoảng trắng.
"""
import json
import re
import sys
import unicodedata
from pathlib import Path

import yaml

CONVERTER_VERSION = 3
KEEP_VALIDITY = {"verified", "preliminary"}
INHERIT = re.compile(r"<([^<>]+)>")
SCRIPT_PREFIX = {  # tên script Hyperglot -> tiền tố tên Unicode của chữ cái
    "Latin": ("LATIN",), "Cyrillic": ("CYRILLIC",), "Greek": ("GREEK",), "Arabic": ("ARABIC",),
    "Hebrew": ("HEBREW",), "Devanagari": ("DEVANAGARI",), "Armenian": ("ARMENIAN",),
    "Georgian": ("GEORGIAN",), "Thai": ("THAI",), "Bengali": ("BENGALI",),
}
MODIFIERS = ("MODIFIER LETTER", "COMBINING", "ZERO WIDTH", "VARIATION")


def load(d: Path):
    langs = {}
    for f in sorted((d / "lib/hyperglot/data").glob("*.yaml")):
        langs[f.stem] = yaml.safe_load(f.read_text(encoding="utf-8"))
    langs["default"] = yaml.safe_load((d / "lib/hyperglot/extra_data/default.yaml").read_text(encoding="utf-8"))
    return langs


def script_ok(script: str, tokens) -> bool:
    pre = SCRIPT_PREFIX.get(script)
    if not pre:
        return True
    for t in tokens:
        for ch in t:
            if not ch.isalpha() or ch.isdigit():
                continue
            name = unicodedata.name(ch, "")
            if name.startswith(MODIFIERS):
                continue
            if not name.startswith(pre):
                return False
    return True


class Resolver:
    def __init__(self, langs):
        self.langs = langs

    def orth(self, code, script, status):
        lang = self.langs.get(code)
        if not lang:
            return None
        cands = [o for o in lang.get("orthographies", []) if (script is None or o.get("script") == script)]
        for o in cands:
            if o.get("status", "primary") == status:
                return o
        return cands[0] if cands and status == "primary" else None

    def get(self, o, attr, script, depth=0):
        raw = (o.get(attr) or "")
        if not isinstance(raw, str):
            raw = " ".join(map(str, raw))
        return self.expand(raw, attr, script, depth)

    def expand(self, raw, attr, script, depth):
        if depth > 8:
            return ""

        def sub(m):
            parts = m.group(1).split()
            code, rest = parts[0], parts[1:]
            a, sc, st = attr, script, "primary"
            for p in rest:
                if p in ("base", "auxiliary", "marks", "punctuation", "numerals", "currency"):
                    a = p
                elif p in ("primary", "secondary", "historical", "transliteration"):
                    st = p
                else:
                    sc = p
            o = self.orth(code, sc, st)
            return self.get(o, a, sc, depth + 1) if o else ""

        return INHERIT.sub(sub, raw)


def nfc(s: str) -> str:
    return unicodedata.normalize("NFC", s)


def nfd_marks(toks):
    return {c for t in toks for c in unicodedata.normalize("NFD", t) if unicodedata.combining(c)}


def required_marks(marks, base, aux):
    """Mark cần cho `base`: có trong NFD của base, hoặc là mark rời không thuộc riêng auxiliary.
    Mark chỉ sinh ra từ ký tự auxiliary (vd é của tiếng Anh) không bắt buộc cho hỗ trợ ngôn ngữ."""
    in_base, in_aux = nfd_marks(base), nfd_marks(aux)
    return [m for m in marks if m in in_base or all(c not in in_aux for c in m)]


def tokens(s: str):
    """Tách grapheme, bỏ ◌, chuẩn hoá NFC (Hyperglot có vài ngôn ngữ ghi NFD)."""
    out = []
    for t in s.split():
        t = nfc(t.replace("◌", ""))
        if t:
            out.append(t)
    return out


def main():
    d, sha = Path(sys.argv[1]), sys.argv[2]
    out = Path(sys.argv[3]) if len(sys.argv) > 3 else Path("langdata.json")
    langs = load(d)
    r = Resolver(langs)
    version = re.search(r"^## (\S+)", (d / "CHANGELOG.md").read_text(encoding="utf-8"), re.M).group(1)
    items, dropped_script, seen = [], 0, {}
    for code, lang in sorted(langs.items()):
        if code == "default" or lang.get("validity") not in KEEP_VALIDITY:
            continue
        name = lang.get("preferred_name") or lang["name"]
        for o in lang.get("orthographies", []):
            if o.get("status", "primary") != "primary":
                continue
            script = o.get("script", "Latin")
            base = tokens(r.get(o, "base", script))
            if not base:
                continue
            if not script_ok(script, base):
                dropped_script += 1
                continue
            marks = tokens(r.get(o, "marks", script))
            aux = tokens(r.get(o, "auxiliary", script))
            marks = required_marks(marks, base, aux)
            n = seen.get(code, 0)
            seen[code] = n + 1
            ident = code if n == 0 else f"{code}-{script}"
            assert ident not in {i["id"] for i in items}, f"id trùng: {ident}"
            items.append({
                "id": ident,
                "bcp47": code,
                "name_en": nfc(name if n == 0 else f"{name} ({script})"),
                "autonym": nfc(o.get("autonym") or lang.get("autonym") or name),
                "script": script,
                "base": " ".join(dict.fromkeys(base)),
                "marks": " ".join(dict.fromkeys(marks)),
            })
    doc = {"hyperglot_sha": sha, "hyperglot_version": version, "converter": CONVERTER_VERSION, "langs": items}
    out.write_text(json.dumps(doc, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    print(f"{len(items)} orthographies, dropped by script check: {dropped_script}, {out.stat().st_size} bytes")


if __name__ == "__main__":
    main()
