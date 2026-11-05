"""Sinh pair.ttc: face 0 = roboto, face 1 = playfair_display. Cần fontTools."""
import pathlib
import sys

from fontTools.ttLib import TTFont
from fontTools.ttLib.ttCollection import TTCollection

core = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
c = TTCollection()
c.fonts = [TTFont(core / "roboto.ttf"), TTFont(core / "playfair_display.ttf")]
c.save(out / "pair.ttc")
