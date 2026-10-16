"""Sinh fixture WOFF1/WOFF2 từ open_sans.ttf. Cần fontTools + brotli."""
import pathlib
import sys

from fontTools.ttLib import TTFont

d = pathlib.Path(sys.argv[1])
for flavor in ("woff", "woff2"):
    f = TTFont(d / "open_sans.ttf")
    f.flavor = flavor
    f.save(d / f"open_sans.{flavor}")
    print("wrote", d / f"open_sans.{flavor}")
