"""
FontAtlas Font Parser
====================

Core module for parsing font files using FontTools and extracting basic metrics.

This module provides OSS-supported features:
- Font file parsing (TTF/OTF)
- Basic metrics extraction (x-height, widths, coverage)
- Glyph outline access via Pen protocol

Phase: 1A - OSS-Supported Features
"""

from fontTools.ttLib import TTFont
from fontTools.pens.recordingPen import RecordingPen
from pathlib import Path
from typing import Dict, List, Optional, Tuple
import json


class FontParser:
    """Parse font files and extract metadata + basic metrics."""
    
    def __init__(self, font_path: str):
        """
        Initialize parser with font file.
        
        Args:
            font_path: Path to TTF/OTF font file
        """
        self.font_path = Path(font_path)
        if not self.font_path.exists():
            raise FileNotFoundError(f"Font file not found: {font_path}")
        
        self.font = TTFont(self.font_path)
        self.font_name = self._get_font_name()
        
    def _get_font_name(self) -> str:
        """Extract human-readable font name from name table."""
        name_table = self.font['name']
        # Try to get full font name (nameID 4)
        for record in name_table.names:
            if record.nameID == 4:  # Full font name
                return record.toUnicode()
        return self.font_path.stem
    
    def get_basic_metrics(self) -> Dict:
        """
        Extract basic OSS-supported metrics.
        
        Returns:
            Dict with metrics:
                - x_height_ratio: x-height / cap height
                - width_variance: std of glyph widths
                - ascender, descender, cap_height
                - units_per_em
                - bounding_box: [xMin, yMin, xMax, yMax]
        """
        # OS/2 table for vertical metrics
        os2_table = self.font.get('OS/2')
        head_table = self.font['head']
        
        # Get heights (normalized by units per em for comparison)
        units_per_em = head_table.unitsPerEm
        
        metrics = {
            'font_name': self.font_name,
            'units_per_em': units_per_em,
        }
        
        if os2_table:
            x_height = os2_table.sxHeight if hasattr(os2_table, 'sxHeight') else None
            cap_height = os2_table.sCapHeight if hasattr(os2_table, 'sCapHeight') else None
            
            # Determine x-height source and value
            x_height_source = None
            optical_x_ratio = None
            
            # Try actual x-height first
            if x_height and cap_height:
                optical_x_ratio = x_height / cap_height
                x_height_source = "actual"
            
            # Fallback 1: estimate from 'x' glyph
            if not x_height:
                x_height = self._estimate_x_height()
                if x_height and cap_height:
                    optical_x_ratio = x_height / cap_height
                    x_height_source = "estimated"
            
            # Fallback 2: estimate cap height if also missing
            if not cap_height:
                cap_height = self._estimate_cap_height()
            
            # Calculate virtual x-height for ALL CAPS
            virtual_x_height = self._estimate_virtual_x_height()
            virtual_x_ratio = virtual_x_height / cap_height if (virtual_x_height and cap_height) else None
            
            # If still no x-height, use virtual as fallback
            if not optical_x_ratio and virtual_x_ratio:
                optical_x_ratio = virtual_x_ratio
                x_height_source = "virtual"
            
            # Last resort default
            if not optical_x_ratio:
                optical_x_ratio = 0.5
                x_height_source = "default"
            
            metrics.update({
                'x_height': x_height,
                'cap_height': cap_height,
                'x_height_ratio': x_height / cap_height if (x_height and cap_height) else None,
                
                # Unified optical metric (for pairing)
                'optical_x_ratio': optical_x_ratio,
                'x_height_source': x_height_source,  # actual | estimated | virtual | default
                
                # Virtual x-height (for ALL CAPS)
                'virtual_x_height': virtual_x_height,
                'virtual_x_ratio': virtual_x_ratio,
                
                'ascender': os2_table.sTypoAscender,
                'descender': os2_table.sTypoDescender,
            })
        
        # Bounding box
        metrics['bounding_box'] = [
            head_table.xMin,
            head_table.yMin,
            head_table.xMax,
            head_table.yMax
        ]
        
        # Width variance (from glyph set)
        widths = self._get_glyph_widths()
        if widths:
            import numpy as np
            metrics['width_mean'] = float(np.mean(widths))
            metrics['width_variance'] = float(np.std(widths))
            metrics['width_min'] = float(np.min(widths))
            metrics['width_max'] = float(np.max(widths))
        
        return metrics
    
    def _get_glyph_widths(self, sample_glyphs: Optional[List[str]] = None) -> List[float]:
        """
        Get advance widths for sample glyphs.
        
        Args:
            sample_glyphs: List of glyph names. If None, uses standard Latin set.
        
        Returns:
            List of normalized widths
        """
        if sample_glyphs is None:
            # Standard Latin lowercase + uppercase for width variance
            sample_glyphs = list('abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ')
        
        hmtx = self.font['hmtx']
        cmap = self.font.getBestCmap()
        units_per_em = self.font['head'].unitsPerEm
        
        widths = []
        for char in sample_glyphs:
            if ord(char) in cmap:
                glyph_name = cmap[ord(char)]
                width, lsb = hmtx[glyph_name]
                # Normalize by units per em
                widths.append(width / units_per_em)
        
        return widths
    
    def _estimate_x_height(self) -> Optional[int]:
        """
        Estimate x-height from lowercase 'x' glyph bounds.
        
        Returns:
            Estimated x-height or None
        """
        try:
            cmap = self.font.getBestCmap()
            if not cmap or ord('x') not in cmap:
                return None
            
            glyph_name = cmap[ord('x')]
            glyph_set = self.font.getGlyphSet()
            
            if glyph_name not in glyph_set:
                return None
            
            # Get glyph bounds using BoundsPen
            try:
                from fontTools.pens.boundsPen import BoundsPen
                glyph = glyph_set[glyph_name]
                pen = BoundsPen(glyph_set)
                glyph.draw(pen)
                bbox = pen.bounds
            except Exception:
                return None
            
            if bbox:
                # x-height is approximately the top of the 'x' glyph
                return int(bbox.yMax)
            
        except Exception:
            pass
        
        return None
    
    def _estimate_cap_height(self) -> Optional[int]:
        """
        Estimate cap height from uppercase 'H' glyph bounds.
        
        Returns:
            Estimated cap height or None
        """
        try:
            cmap = self.font.getBestCmap()
            if not cmap or ord('H') not in cmap:
                return None
            
            glyph_name = cmap[ord('H')]
            glyph_set = self.font.getGlyphSet()
            
            if glyph_name not in glyph_set:
                return None
            
            # Get glyph bounds
            glyph = glyph_set[glyph_name]
            bbox = glyph._glyph.getBounds(glyph_set)
            
            if bbox:
                # Cap height is the top of the 'H' glyph
                return int(bbox.yMax)
            
        except Exception:
            pass
        
        return None
    
    def _estimate_virtual_x_height(self) -> Optional[int]:
        """
        Estimate virtual x-height for ALL CAPS fonts.
        
        Virtual x-height = optical mass center of uppercase letters,
        excluding overshoots and terminals.
        
        This is crucial for:
        - ALL CAPS font pairing
        - Optical size matching
        - Display font sizing
        
        Returns:
            Estimated virtual x-height or None
        """
        try:
            cmap = self.font.getBestCmap()
            if not cmap:
                return None
            
            glyph_set = self.font.getGlyphSet()
            
            # Sample uppercase letters with strong horizontal mass
            # H = strong vertical, O = rounded, N = diagonal
            test_chars = ['H', 'O', 'N']
            masses = []
            
            for char in test_chars:
                if ord(char) not in cmap:
                    continue
                
                glyph_name = cmap[ord(char)]
                if glyph_name not in glyph_set:
                    continue
                
                try:
                    from fontTools.pens.boundsPen import BoundsPen
                    glyph = glyph_set[glyph_name]
                    pen = BoundsPen(glyph_set)
                    glyph.draw(pen)
                    bbox = pen.bounds
                    
                    if not bbox:
                        continue
                    
                    height = bbox.yMax - bbox.yMin
                    
                    # Exclude top/bottom 12% (overshoots, terminals)
                    # The remaining 76% is the optical mass
                    exclude_ratio = 0.12
                    optical_bottom = bbox.yMin + (height * exclude_ratio)
                    optical_top = bbox.yMax - (height * exclude_ratio)
                    
                    # Virtual x-height is the top of optical mass
                    masses.append(optical_top)
                    
                except Exception:
                    continue
            
            if masses:
                # Average optical mass top across test glyphs
                return int(sum(masses) / len(masses))
            
        except Exception:
            pass
        
        return None
    
    def get_unicode_coverage(self) -> Dict:
        """
        Analyze Unicode coverage for language support.
        
        Returns:
            Dict with:
                - total_glyphs: int
                - unicode_ranges: Dict[range_name, coverage_percentage]
                - supports_vietnamese: bool
                - supports_latin_extended: bool
        """
        cmap = self.font.getBestCmap()
        if not cmap:
            return {'total_glyphs': 0, 'unicode_ranges': {}}
        
        codepoints = set(cmap.keys())
        
        # Define key Unicode ranges
        unicode_ranges = {
            'Basic Latin': (0x0020, 0x007F),
            'Latin-1 Supplement': (0x0080, 0x00FF),
            'Latin Extended-A': (0x0100, 0x017F),
            'Latin Extended-B': (0x0180, 0x024F),
            'Vietnamese': self._vietnamese_codepoints(),
        }
        
        coverage = {}
        for range_name, range_def in unicode_ranges.items():
            if isinstance(range_def, tuple):
                start, end = range_def
                range_cps = set(range(start, end + 1))
            else:
                range_cps = range_def  # Already a set (for Vietnamese)
            
            present = codepoints.intersection(range_cps)
            coverage[range_name] = len(present) / len(range_cps) if range_cps else 0
        
        return {
            'total_glyphs': len(codepoints),
            'unicode_ranges': coverage,
            'supports_vietnamese': coverage.get('Vietnamese', 0) > 0.9,
            'supports_latin_extended': coverage.get('Latin Extended-A', 0) > 0.8,
        }
    
    @staticmethod
    def _vietnamese_codepoints() -> set:
        """Return set of essential Vietnamese codepoints (with diacritics)."""
        # Essential Vietnamese characters with tone marks
        vietnamese = set()
        # Vowels with tone marks (combining diacritics)
        base_vowels = 'aăâeêioôơuưy'
        for char in base_vowels + base_vowels.upper():
            vietnamese.add(ord(char))
        
        # Add common Vietnamese combining marks
        combining_marks = [
            0x0300,  # Grave
            0x0301,  # Acute
            0x0303,  # Tilde
            0x0309,  # Hook above
            0x0323,  # Dot below
        ]
        vietnamese.update(combining_marks)
        
        # Precomposed Vietnamese characters
        vietnamese.update(range(0x00C0, 0x00FF))  # Latin-1 Supplement (includes many Vietnamese)
        vietnamese.update(range(0x0102, 0x0103))  # Ă ă
        vietnamese.update(range(0x00D4, 0x00D5))  # Ô Õ
        vietnamese.update(range(0x01A0, 0x01B0))  # Ơ ơ Ư ư
        
        return vietnamese
    
    def get_glyph_outline(self, glyph_name: str) -> List:
        """
        Get Bezier outline data for a glyph.
        
        Args:
            glyph_name: Name of the glyph (e.g., 'A', 'a')
        
        Returns:
            List of drawing commands from RecordingPen
        """
        glyph_set = self.font.getGlyphSet()
        if glyph_name not in glyph_set:
            return []
        
        pen = RecordingPen()
        glyph_set[glyph_name].draw(pen)
        return pen.value
    
    def get_glyph_outline_for_char(self, char: str) -> List:
        """
        Get outline for a character (convenience wrapper).
        
        Args:
            char: Single character (e.g., 'A')
        
        Returns:
            Outline commands
        """
        cmap = self.font.getBestCmap()
        if ord(char) not in cmap:
            return []
        
        glyph_name = cmap[ord(char)]
        return self.get_glyph_outline(glyph_name)
    
    def extract_full_profile(self) -> Dict:
        """
        Extract complete font profile (Phase 1A).
        
        Returns:
            Dict with basic metrics + coverage
        """
        profile = {
            'metadata': {
                'font_name': self.font_name,
                'file_path': str(self.font_path),
            },
            'basic_metrics': self.get_basic_metrics(),
            'unicode_coverage': self.get_unicode_coverage(),
        }
        
        return profile
    
    def save_profile(self, output_path: str):
        """Save font profile to JSON."""
        profile = self.extract_full_profile()
        with open(output_path, 'w', encoding='utf-8') as f:
            json.dump(profile, f, indent=2, ensure_ascii=False)
        print(f"✓ Saved profile to {output_path}")


def main():
    """CLI for testing font parser."""
    import sys
    
    if len(sys.argv) < 2:
        print("Usage: python font_parser.py <font_file.ttf>")
        sys.exit(1)
    
    font_path = sys.argv[1]
    
    try:
        parser = FontParser(font_path)
        print(f"\n{'='*60}")
        print(f"Font: {parser.font_name}")
        print(f"{'='*60}\n")
        
        # Basic metrics
        metrics = parser.get_basic_metrics()
        print("📏 Basic Metrics:")
        print(f"  x-height ratio: {metrics.get('x_height_ratio', 'N/A'):.3f}" if metrics.get('x_height_ratio') else "  x-height ratio: N/A")
        print(f"  Width variance: {metrics.get('width_variance', 'N/A'):.3f}" if metrics.get('width_variance') else "  Width variance: N/A")
        print(f"  Units per em: {metrics['units_per_em']}")
        
        # Coverage
        coverage = parser.get_unicode_coverage()
        print(f"\n🌍 Unicode Coverage:")
        print(f"  Total glyphs: {coverage['total_glyphs']}")
        print(f"  Vietnamese support: {'✓' if coverage['supports_vietnamese'] else '✗'}")
        for range_name, pct in coverage['unicode_ranges'].items():
            print(f"  {range_name}: {pct*100:.1f}%")
        
        # Save full profile
        output_path = font_path.replace('.ttf', '_profile.json').replace('.otf', '_profile.json')
        parser.save_profile(output_path)
        
    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == '__main__':
    main()
