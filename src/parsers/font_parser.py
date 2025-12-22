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
        
    def _get_name_record(self, name_id: int) -> Optional[str]:
        """Helper to get name record by ID."""
        for record in self.font['name'].names:
            if record.nameID == name_id:
                return record.toUnicode()
        return None

    def _get_font_name(self) -> str:
        """Extract human-readable font name from name table."""
        # Try ID 4 (Full Name) -> ID 1 (Family Name)
        name = self._get_name_record(4) or self._get_name_record(1) or self.font_path.stem
        return name

    def get_metadata(self) -> Dict:
        """Extract explicit metadata."""
        return {
            'foundry': self._get_name_record(8),  # Manufacturer
            'designer': self._get_name_record(9), # Designer
            'license': self._get_name_record(13), # License
            'version': self._get_name_record(5),  # Version string
            'copyright': self._get_name_record(0), # Copyright
        }

    def get_weight_style(self) -> Dict:
        """Extract weight and style information."""
        os2 = self.font.get('OS/2')
        head = self.font.get('head')
        post = self.font.get('post')

        # Weight
        weight = 400
        if os2:
            weight = os2.usWeightClass

        # Italic / Slant
        is_italic = False
        slant_angle = 0
        
        if head:
           if (head.macStyle & 2): # Bit 1 is italic
               is_italic = True
        
        if os2 and not is_italic:
            if (os2.fsSelection & 1): # Bit 0 is italic
                is_italic = True
        
        if post:
            slant_angle = post.italicAngle
            if slant_angle != 0 and not is_italic:
                is_italic = True

        return {
            'weight': weight,
            'is_italic': is_italic,
            'slant_angle': slant_angle
        }

    def get_family_info(self) -> Dict:
        """Extract font family information for grouping."""
        os2 = self.font.get('OS/2')
        
        # Family name (ID 1) and Subfamily (ID 2)
        family_name = self._get_name_record(1) or self.font_path.stem
        subfamily = self._get_name_record(2) or 'Regular'
        
        # Weight class from OS/2 table
        weight_class = 400  # Default to Regular
        if os2:
            weight_class = os2.usWeightClass
        
        # Width class from OS/2 table
        width_class = 5  # Default to Medium (5)
        if os2 and hasattr(os2, 'usWidthClass'):
            width_class = os2.usWidthClass
        
        return {
            'family_name': family_name,
            'subfamily': subfamily,
            'weight_class': weight_class,
            'width_class': width_class
        }

    def get_panose_data(self) -> Optional[List[int]]:
        """Extract PANOSE classification numbers."""
        os2 = self.font.get('OS/2')
        if os2 and hasattr(os2, 'panose'):
            # panose is an object with search capability only in some versions of fontTools?
            # usually it has bFamilyType, bSerifStyle, etc.
            # We want the raw 10 bytes if possible, or mapped values.
            try:
                p = os2.panose
                return [
                    p.bFamilyType,
                    p.bSerifStyle,
                    p.bWeight,
                    p.bProportion,
                    p.bContrast,
                    p.bStrokeVariation,
                    p.bArmStyle,
                    p.bLetterForm,
                    p.bMidline,
                    p.bXHeight
                ]
            except:
                return None
        return None
    
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
    
    def get_anatomy_metrics(self) -> Dict:
        """
        Extract advanced anatomical metrics for accurate classification.
        
        Returns:
            Dict with:
                - terminal_sharpness: 0.0 (blunt) to 1.0 (sharp)
                - stress_angle_degrees: 0°-90° angle of stress
                - stroke_contrast: ratio of thickest/thinnest stroke
                - aperture: 0.0 (closed) to 1.0 (wide open)
                - curvature_ratio: 0.0 (geometric) to 1.0 (humanist)
                - counter_density: 0.0 (tight) to 1.0 (spacious)
        """
        return {
            'terminal_sharpness': self._calculate_terminal_sharpness(),
            'stress_angle': self._calculate_stress_angle(),
            'stroke_contrast': self._calculate_stroke_contrast(),
            'aperture': self._calculate_aperture_openness(),
            'curvature_ratio': self._calculate_curvature_distribution(),
            'counter_density': self._calculate_counter_density(),
        }
    
    def _calculate_terminal_sharpness(self) -> float:
        """
        Calculate terminal sharpness by analyzing endings of characters like 'a', 'c', 'e'.
        Sharp terminals end abruptly, blunt terminals have rounded endings.
        
        Returns:
            0.0 = blunt/rounded terminals, 1.0 = sharp/cut terminals
        """
        try:
            from fontTools.pens.boundsPen import BoundsPen
            import numpy as np
            
            cmap = self.font.getBestCmap()
            if not cmap:
                return 0.5
            
            glyph_set = self.font.getGlyphSet()
            test_chars = ['a', 'c', 'e', 'r']  # Characters with terminals
            sharpness_scores = []
            
            for char in test_chars:
                if ord(char) not in cmap:
                    continue
                
                glyph_name = cmap[ord(char)]
                if glyph_name not in glyph_set:
                    continue
                
                outline = self.get_glyph_outline(glyph_name)
                if not outline:
                    continue
                
                # Analyze terminal points (end of curves)
                curve_endpoints = 0
                sharp_endpoints = 0
                
                for cmd, args in outline:
                    if cmd in ('curveTo', 'qCurveTo'):
                        curve_endpoints += 1
                        # Check if the curve ends sharply (abrupt direction change)
                        # A sharp terminal will have the control point far from the endpoint
                        if cmd == 'curveTo' and len(args) >= 3:
                            cp1, cp2, end = args[0], args[1], args[2]
                            # Calculate angle at endpoint
                            dx = end[0] - cp2[0]
                            dy = end[1] - cp2[1]
                            length = np.sqrt(dx**2 + dy**2)
                            if length > 50:  # Significant control point distance = sharp
                                sharp_endpoints += 1
                
                if curve_endpoints > 0:
                    sharpness_scores.append(sharp_endpoints / curve_endpoints)
            
            if sharpness_scores:
                return float(np.mean(sharpness_scores))
            
            return 0.5  # Default: moderate
            
        except Exception:
            return 0.5
    
    def _calculate_stress_angle(self) -> float:
        """
        Measure stress angle in rounded characters like 'o', 'e', 'b'.
        Stress angle is the angle of the thickest point from vertical.
        
        Returns:
            Angle in degrees (0° = vertical, 90° = horizontal)
        """
        try:
            import numpy as np
            from fontTools.pens.boundsPen import BoundsPen
            
            cmap = self.font.getBestCmap()
            if not cmap or ord('o') not in cmap:
                return 0.0
            
            glyph_set = self.font.getGlyphSet()
            glyph_name = cmap[ord('o')]
            
            if glyph_name not in glyph_set:
                return 0.0
            
            outline = self.get_glyph_outline(glyph_name)
            if not outline:
                return 0.0
            
            # Find the thickest horizontal section of 'o'
            # This is a simplified heuristic: measure the widest vertical cross-section
            pen = BoundsPen(glyph_set)
            glyph_set[glyph_name].draw(pen)
            bbox = pen.bounds
            
            if not bbox:
                return 0.0
            
            # Approximate stress angle from bbox shape
            # If bbox is wider than tall, stress is more horizontal
            # If taller than wide, stress is more vertical
            width = bbox.xMax - bbox.xMin
            height = bbox.yMax - bbox.yMin
            
            if height == 0:
                return 90.0
            
            # Simple heuristic: aspect ratio maps to stress angle
            # 1:1 ratio = ~45°, wider = higher angle
            aspect_ratio = width / height
            angle = min(90.0, 30.0 + (aspect_ratio - 0.7) * 40.0)
            
            return max(0.0, angle)
            
        except Exception:
            return 0.0
    
    def _calculate_stroke_contrast(self) -> float:
        """
        Calculate stroke contrast ratio (thickest stroke / thinnest stroke).
        Analyzes the 'O' character to find maximum and minimum stroke widths.
        
        Returns:
            Ratio (1.0 = monoweight, 10+ = high contrast like Didone)
        """
        try:
            import numpy as np
            from fontTools.pens.boundsPen import BoundsPen
            
            cmap = self.font.getBestCmap()
            if not cmap or ord('O') not in cmap:
                return 1.0
            
            glyph_set = self.font.getGlyphSet()
            glyph_name = cmap[ord('O')]
            
            if glyph_name not in glyph_set:
                return 1.0
            
            outline = self.get_glyph_outline(glyph_name)
            if not outline:
                return 1.0
            
            # Measure stroke widths by analyzing the outline points
            # This is a simplified approach: sample vertical and horizontal sections
            pen = BoundsPen(glyph_set)
            glyph_set[glyph_name].draw(pen)
            bbox = pen.bounds
            
            if not bbox:
                return 1.0
            
            # Sample stroke widths at different points
            # For a proper implementation, we'd scan horizontally and vertically
            # For now, use a heuristic based on outline complexity
            
            # Count line segments vs curves - more curves = potentially higher contrast
            lines = sum(1 for cmd, _ in outline if cmd == 'lineTo')
            curves = sum(1 for cmd, _ in outline if cmd in ('curveTo', 'qCurveTo'))
            
            if lines + curves == 0:
                return 1.0
            
            # Simple heuristic: more curves relative to lines suggests contrast
            # This is not accurate but better than placeholder 0
            curve_ratio = curves / (lines + curves + 1)
            
            # Map curve ratio to contrast
            # High curve ratio (0.8+) = serif with contrast (2.0-4.0)
            # Low curve ratio (0.3-) = geometric sans (1.0-1.5)
            if curve_ratio > 0.7:
                contrast = 2.0 + (curve_ratio - 0.7) * 5.0  # 2.0 to 3.5
            else:
                contrast = 1.0 + curve_ratio  # 1.0 to 1.7
            
            return min(10.0, contrast)
            
        except Exception:
            return 1.0
    
    def _calculate_aperture_openness(self) -> float:
        """
        Calculate aperture openness by measuring the opening width of characters
        like 'c', 'e', 's' relative to their total width.
        
        Returns:
            0.0 = closed/narrow aperture, 1.0 = wide open aperture
        """
        try:
            import numpy as np
            from fontTools.pens.boundsPen import BoundsPen
            
            cmap = self.font.getBestCmap()
            if not cmap:
                return 0.5
            
            glyph_set = self.font.getGlyphSet()
            test_chars = ['c', 'e', 's']  # Characters with apertures
            aperture_scores = []
            
            for char in test_chars:
                if ord(char) not in cmap:
                    continue
                
                glyph_name = cmap[ord(char)]
                if glyph_name not in glyph_set:
                    continue
                
                pen = BoundsPen(glyph_set)
                glyph_set[glyph_name].draw(pen)
                bbox = pen.bounds
                
                if not bbox:
                    continue
                
                width = bbox.xMax - bbox.xMin
                height = bbox.yMax - bbox.yMin
                
                if width == 0:
                    continue
                
                # Heuristic: wider characters tend to have more open apertures
                # Compare to x-height to normalize
                x_height = self.font.get('OS/2').sxHeight if self.font.get('OS/2') else height
                
                if x_height > 0:
                    relative_width = width / x_height
                    # Map relative width to aperture openness
                    # Narrow (0.4-0.6) = closed, Wide (0.8-1.0) = open
                    aperture = (relative_width - 0.4) / 0.6
                    aperture_scores.append(max(0.0, min(1.0, aperture)))
            
            if aperture_scores:
                return float(np.mean(aperture_scores))
            
            return 0.5  # Default: moderate
            
        except Exception:
            return 0.5
    
    def _calculate_curvature_distribution(self) -> float:
        """
        Analyze distribution of curved vs straight segments in the font.
        Geometric fonts have more straight segments, humanist fonts more curves.
        
        Returns:
            0.0 = geometric (many straight lines), 1.0 = humanist (many curves)
        """
        try:
            cmap = self.font.getBestCmap()
            if not cmap:
                return 0.5
            
            glyph_set = self.font.getGlyphSet()
            test_chars = list('abcdefghijklmnopqrstuvwxyz')  # Sample lowercase
            
            total_lines = 0
            total_curves = 0
            
            for char in test_chars:
                if ord(char) not in cmap:
                    continue
                
                glyph_name = cmap[ord(char)]
                if glyph_name not in glyph_set:
                    continue
                
                outline = self.get_glyph_outline(glyph_name)
                
                for cmd, _ in outline:
                    if cmd == 'lineTo':
                        total_lines += 1
                    elif cmd in ('curveTo', 'qCurveTo'):
                        total_curves += 1
            
            if total_lines + total_curves == 0:
                return 0.5
            
            # Ratio of curves to total segments
            curve_ratio = total_curves / (total_lines + total_curves)
            
            return float(curve_ratio)
            
        except Exception:
            return 0.5
    
    def _calculate_counter_density(self) -> float:
        """
        Calculate white space density within character counters (internal spaces).
        Analyzes characters like 'o', 'p', 'g' to measure counter size vs glyph size.
        
        Returns:
            0.0 = tight/small counters, 1.0 = spacious/large counters
        """
        try:
            import numpy as np
            from fontTools.pens.boundsPen import BoundsPen
            
            cmap = self.font.getBestCmap()
            if not cmap:
                return 0.5
            
            glyph_set = self.font.getGlyphSet()
            test_chars = ['o', 'p', 'g', 'b', 'd']  # Characters with clear counters
            density_scores = []
            
            for char in test_chars:
                if ord(char) not in cmap:
                    continue
                
                glyph_name = cmap[ord(char)]
                if glyph_name not in glyph_set:
                    continue
                
                pen = BoundsPen(glyph_set)
                glyph_set[glyph_name].draw(pen)
                bbox = pen.bounds
                
                if not bbox:
                    continue
                
                # Calculate bbox area
                width = bbox.xMax - bbox.xMin
                height = bbox.yMax - bbox.yMin
                bbox_area = width * height
                
                if bbox_area == 0:
                    continue
                
                # Estimate counter area (very simplified)
                # For 'o', the counter is roughly 40-60% of bbox for normal fonts
                # Tight fonts: 30-40%, Spacious fonts: 60-80%
                
                # Heuristic: use stroke contrast as proxy
                # Lower contrast = thinner strokes = larger counters
                outline = self.get_glyph_outline(glyph_name)
                lines = sum(1 for cmd, _ in outline if cmd == 'lineTo')
                curves = sum(1 for cmd, _ in outline if cmd in ('curveTo', 'qCurveTo'))
                
                if lines + curves > 0:
                    curve_ratio = curves / (lines + curves)
                    # More geometric (low curves) = larger counters typically
                    # More curves = potentially tighter from stroke variation
                    density = 1.0 - curve_ratio * 0.5  # Inverse relationship
                    density_scores.append(max(0.0, min(1.0, density)))
            
            if density_scores:
                return float(np.mean(density_scores))
            
            return 0.5  # Default: moderate
            
        except Exception:
            return 0.5
    
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
    
    def detect_serifs(self) -> Dict:
        """
        Detect whether font has serifs by analyzing outline complexity
        of simple vertical characters like 'I' and 'l'.
        
        Method: Count outline commands. Sans 'I' has ~5 commands (simple rectangle),
        while serif 'I' has 12+ (rectangle + serifs at top and bottom).
        
        Returns:
            Dict with:
                - has_serifs: bool (True = serif font, False = sans-serif)
                - confidence: float (0.0-1.0)
                - command_count: average commands for analyzed glyphs
                - analyzed_chars: list of chars successfully analyzed
        """
        try:
            cmap = self.font.getBestCmap()
            if not cmap:
                return {'has_serifs': None, 'confidence': 0.0, 'command_count': 0, 'analyzed_chars': []}
            
            # Test characters: simple verticals
            # 'I' is best - just a vertical stroke that clearly shows serif presence
            # 'l' (lowercase L) as backup
            test_chars = ['I', 'l']
            command_counts = []
            analyzed_chars = []
            
            for char in test_chars:
                if ord(char) not in cmap:
                    continue
                
                outline = self.get_glyph_outline_for_char(char)
                if not outline or len(outline) < 3:
                    continue
                
                cmd_count = len(outline)
                command_counts.append(cmd_count)
                analyzed_chars.append(char)
            
            if not command_counts:
                # Fallback: try other vertical chars
                for char in ['H', 'n', 't']:
                    if ord(char) not in cmap:
                        continue
                    outline = self.get_glyph_outline_for_char(char)
                    if outline:
                        # For H/n, divide by 2 since they have more complexity
                        adjusted_count = len(outline) / 2
                        command_counts.append(adjusted_count)
                        analyzed_chars.append(char)
            
            if not command_counts:
                return {'has_serifs': None, 'confidence': 0.0, 'command_count': 0, 'analyzed_chars': []}
            
            import numpy as np
            avg_count = np.mean(command_counts)
            
            # Thresholds based on analysis:
            # Sans 'I': 4-6 commands (simple rectangle)
            # Serif 'I': 10-15+ commands (rectangle + serifs)
            # Decorative/Script: 8-12+ with curves
            
            if avg_count <= 6:
                has_serifs = False  # Simple, clean = sans
                confidence = min(1.0, (7 - avg_count) / 3)  # Higher confidence for fewer commands
            elif avg_count >= 10:
                has_serifs = True   # Complex = has serifs/decorations
                confidence = min(1.0, (avg_count - 9) / 5)
            else:
                # Ambiguous zone (7-9 commands) - likely decorative sans or light serif
                has_serifs = None
                confidence = 0.3
            
            return {
                'has_serifs': has_serifs,
                'confidence': float(confidence),
                'command_count': float(avg_count),
                'analyzed_chars': analyzed_chars
            }
            
        except Exception as e:
            return {'has_serifs': None, 'confidence': 0.0, 'command_count': 0, 'error': str(e), 'analyzed_chars': []}

    
    def _analyze_serif_presence(self, glyph_name: str, glyph_set) -> Optional[float]:
        """
        Analyze a single glyph for serif presence.
        
        Method: Compare width at baseline/terminal regions vs stem width.
        Serifs cause significant width increase at terminals.
        
        Returns:
            Serif ratio (terminal_width / stem_width), or None if analysis fails
            > 1.3 indicates serifs present
        """
        try:
            from fontTools.pens.boundsPen import BoundsPen
            import numpy as np
            
            # Get overall bounds
            pen = BoundsPen(glyph_set)
            glyph_set[glyph_name].draw(pen)
            bbox = pen.bounds
            
            if not bbox:
                return None
            
            # Get outline commands
            outline = self.get_glyph_outline(glyph_name)
            if not outline:
                return None
            
            # Collect all points from outline
            points = []
            current = (0, 0)
            
            for cmd, args in outline:
                if cmd == 'moveTo':
                    current = args[0]
                    points.append(current)
                elif cmd == 'lineTo':
                    current = args[0]
                    points.append(current)
                elif cmd == 'curveTo':
                    # Add control points and end point
                    for pt in args:
                        points.append(pt)
                    current = args[-1]
                elif cmd == 'qCurveTo':
                    for pt in args:
                        points.append(pt)
                    current = args[-1]
            
            if len(points) < 4:
                return None
            
            # Analyze horizontal extent at different y-levels
            height = bbox[3] - bbox[1]  # yMax - yMin
            if height <= 0:
                return None
            
            # Sample horizontal widths at different heights
            # Baseline region: bottom 15%
            # Middle (stem) region: 35-65%
            # Cap region: top 15%
            baseline_y_max = bbox[1] + height * 0.15
            middle_y_min = bbox[1] + height * 0.35
            middle_y_max = bbox[1] + height * 0.65
            cap_y_min = bbox[3] - height * 0.15
            
            def get_x_extent_in_region(y_min, y_max):
                """Get min/max X coordinates of points in y-region."""
                region_points = [p for p in points if y_min <= p[1] <= y_max]
                if not region_points:
                    return None
                xs = [p[0] for p in region_points]
                return max(xs) - min(xs) if xs else None
            
            baseline_width = get_x_extent_in_region(bbox[1], baseline_y_max)
            stem_width = get_x_extent_in_region(middle_y_min, middle_y_max)
            cap_width = get_x_extent_in_region(cap_y_min, bbox[3])
            
            if not stem_width or stem_width <= 0:
                return None
            
            # Calculate ratios
            ratios = []
            if baseline_width and baseline_width > 0:
                ratios.append(baseline_width / stem_width)
            if cap_width and cap_width > 0:
                ratios.append(cap_width / stem_width)
            
            if not ratios:
                return None
            
            return float(np.mean(ratios))
            
        except Exception:
            return None
    
    def detect_symmetry(self) -> Dict:
        """
        Detect vertical symmetry of uppercase letters H, A, I.
        
        Sans/Serif fonts have near-perfect symmetry (>0.95)
        Script/Display fonts have lower symmetry (<0.92)
        
        Returns:
            Dict with:
                - symmetry_score: float (0-1, average symmetry)
                - is_symmetric: bool (True if score > 0.92)
                - characters: dict of per-character scores
        """
        import numpy as np
        
        def calculate_char_symmetry(char: str) -> Optional[float]:
            """Calculate vertical symmetry score for a character."""
            outline = self.get_glyph_outline_for_char(char)
            if not outline or len(outline) < 4:
                return None
            
            # Collect all on-curve points
            points = []
            for cmd, args in outline:
                if cmd in ('moveTo', 'lineTo'):
                    points.append(args[0])
                elif cmd in ('curveTo', 'qCurveTo'):
                    points.append(args[-1])  # End point
            
            if len(points) < 4:
                return None
            
            # Find center X axis
            xs = [p[0] for p in points]
            center_x = (max(xs) + min(xs)) / 2
            width = max(xs) - min(xs)
            
            if width <= 0:
                return None
            
            # Split into left and right halves
            left_points = [(p[0], p[1]) for p in points if p[0] < center_x]
            right_points = [(p[0], p[1]) for p in points if p[0] > center_x]
            
            if len(left_points) < 2 or len(right_points) < 2:
                return None
            
            # Mirror right points around center axis
            mirrored_right = [(2 * center_x - p[0], p[1]) for p in right_points]
            
            # Sort by Y for comparison
            left_sorted = sorted(left_points, key=lambda p: p[1])
            right_sorted = sorted(mirrored_right, key=lambda p: p[1])
            
            # Calculate average distance between corresponding points
            min_len = min(len(left_sorted), len(right_sorted))
            if min_len < 2:
                return None
            
            total_diff = 0
            for i in range(min_len):
                left_p = left_sorted[i]
                # Find nearest mirrored point
                min_dist = float('inf')
                for right_p in right_sorted:
                    dist = np.sqrt((left_p[0] - right_p[0])**2 + (left_p[1] - right_p[1])**2)
                    min_dist = min(min_dist, dist)
                total_diff += min_dist
            
            # Normalize by glyph width
            symmetry_score = 1.0 - (total_diff / (min_len * width))
            return max(0.0, min(1.0, symmetry_score))
        
        # Test symmetry on uppercase letters
        test_chars = ['H', 'A', 'I']
        char_scores = {}
        
        for char in test_chars:
            score = calculate_char_symmetry(char)
            if score is not None:
                char_scores[char] = score
        
        if not char_scores:
            return {
                'symmetry_score': 0.5,
                'is_symmetric': None,
                'characters': {}
            }
        
        avg_symmetry = float(np.mean(list(char_scores.values())))
        
        return {
            'symmetry_score': round(avg_symmetry, 3),
            'is_symmetric': avg_symmetry > 0.92,
            'characters': char_scores
        }

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
            'anatomy_metrics': self.get_anatomy_metrics(),
            'unicode_coverage': self.get_unicode_coverage(),
            'metadata': self.get_metadata(), # Override/Extend standard metadata
            'weight_style': self.get_weight_style(),
            'family_info': self.get_family_info(),
            'panose': self.get_panose_data(),
            'serif_detection': self.detect_serifs(),  # Direct serif detection
            'symmetry_detection': self.detect_symmetry(),  # NEW: H, A, I symmetry
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
