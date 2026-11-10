#!/usr/bin/env python3
"""Đo x-height của hai font bundled dùng cho wordmark "Font|Atlas" của splash → UX/components/splash/wordmarkMetrics.ts.

x-height = đỉnh PHẲNG của glyph 'x' (không phải 'o', có overshoot) ở đúng wght sẽ dùng, theo đơn vị font. Hard-code vào TS; test kiểm lại tỉ lệ.
Chạy:  /tmp/v/bin/python tools/design/measure_wordmark_fonts.py   (venv có fonttools brotli)
"""
from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
from fontTools.pens.boundsPen import BoundsPen

ROOT = Path(__file__).resolve().parents[2]
FONTS = ROOT / "UX/public/fonts"
OUT = ROOT / "UX/components/splash/wordmarkMetrics.ts"
SPEC = [("SERIF", "Newsreader", "newsreader-latin-wght-normal.woff2", 500, "OFL-Newsreader.txt"),
        ("SANS", "Inter", "inter-latin-wght-normal.woff2", 600, "OFL-Inter.txt"),
        ("MONO", "JetBrains Mono", "jetbrains-mono-latin-wght-normal.woff2", 500, "OFL-JetBrainsMono.txt")]
rows = []
for const, family, file, wght, lic in SPEC:
    t = instantiateVariableFont(TTFont(FONTS / file), {"wght": wght})
    gs, cm = t.getGlyphSet(), t.getBestCmap()
    assert all(ord(c) in cm for c in "FontAtlas"), family
    bp = BoundsPen(gs); gs[cm[ord("x")]].draw(bp)
    rows.append(f"/** {family} {wght}, đỉnh phẳng của 'x' = {round(bp.bounds[3])}/{t['head'].unitsPerEm} em. Giấy phép: public/fonts/{lic}. */\n"
                f"export const {const}: WordmarkFont = {{ family: '{family}', weight: {wght}, upm: {t['head'].unitsPerEm}, xHeight: {round(bp.bounds[3])} }};")
OUT.write_text("// Tạo bởi tools/design/measure_wordmark_fonts.py. Không sửa tay.\nexport type WordmarkFont = { family: string; weight: number; upm: number; xHeight: number };\n" + "\n".join(rows) + "\n")
print(OUT.read_text())
