#!/usr/bin/env python3
"""Trích đường viền glyph Inter (OFL, đã đóng gói trong UX/public/fonts) thành UX/components/onboarding/glyphOutlines.ts.

Dùng cho hình minh hoạ onboarding (blueprint). Không dùng font của người dùng.
Chạy:  python3 -m venv /tmp/v && /tmp/v/bin/pip install fonttools brotli skia-pathops && /tmp/v/bin/python tools/design/extract_glyph_outlines.py
Trục y lật xuống: baseline y=0, chữ hoa lên trên là y âm. wght cố định 500.
"""
from pathlib import Path
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
from fontTools.ttLib.removeOverlaps import removeOverlaps
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.boundsPen import BoundsPen

ROOT = Path(__file__).resolve().parents[2]
FONTS = ROOT / "UX/public/fonts"
OUT = ROOT / "UX/components/onboarding/glyphOutlines.ts"
WANT = {"latin": "AaghHxêl", "vietnamese": "Ặựẫă"}
rows = []
metrics = {}
for sub, chars in WANT.items():
    t = instantiateVariableFont(TTFont(FONTS / f"inter-{sub}-wght-normal.woff2"), {"wght": 500})
    removeOverlaps(t)  # contour chồng nhau (A = hai chân + thanh ngang) sẽ lộ viền thừa khi vẽ nét; cần skia-pathops
    gs, cm = t.getGlyphSet(), t.getBestCmap()
    os2 = t["OS/2"]
    metrics = {"upm": t["head"].unitsPerEm, "xHeight": os2.sxHeight, "capHeight": os2.sCapHeight}
    for ch in chars:
        g = gs[cm[ord(ch)]]
        sp = SVGPathPen(gs, ntos=lambda v: str(int(round(v))))
        g.draw(TransformPen(sp, (1, 0, 0, -1, 0, 0)))
        bp = BoundsPen(gs); g.draw(bp)
        x0, y0, x1, y1 = (int(round(v)) for v in bp.bounds)
        rows.append(f"  {ch!r}: {{ d: '{sp.getCommands()}', adv: {g.width}, x0: {x0}, x1: {x1}, top: {y1}, bottom: {y0} }},".replace('"', "'"))
body = "\n".join(rows)
OUT.write_text(f"""// Tạo bởi tools/design/extract_glyph_outlines.py từ Inter (SIL OFL 1.1), wght 500. Không sửa tay.
// Toạ độ theo đơn vị font (upm {metrics['upm']}), y hướng xuống: baseline y=0, cap = -{metrics['capHeight']}, x-height = -{metrics['xHeight']}.
export const UPM = {metrics['upm']};
export const X_HEIGHT = {metrics['xHeight']};
export const CAP_HEIGHT = {metrics['capHeight']};
export type GlyphOutline = {{ d: string; adv: number; x0: number; x1: number; top: number; bottom: number }};
export const GLYPHS: Readonly<Record<string, GlyphOutline>> = {{
{body}
}};
""")
print("wrote", OUT, len(body))
