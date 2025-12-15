"""
Aperture Detector
==================

Measure aperture openness - the width of partially enclosed negative space.

Apertures are critical for legibility at small sizes. Wider apertures (Helvetica)
are more readable than narrow ones (Futura) in body text.

Key glyphs: c, e, s, n, a

Algorithm:
1. Extract glyph contours from FontTools
2. Identify inner vs outer contours
3. Find counter openings (endpoints that don't close)
4. Measure distance between opening endpoints
5. Normalize by x-height for scale-independence

Classifications:
- Wide aperture (>0.6): Good for small text (Helvetica, Roboto)
- Moderate (0.4-0.6): Balanced (Georgia, Verdana)
- Narrow (<0.4): Display fonts (Futura, Avant Garde)
"""

from pathlib import Path
from typing import Dict, List, Tuple, Optional
import numpy as np
from fontTools.ttLib import TTFont
from fontTools.pens.recordingPen import RecordingPen
from fontTools.pens.boundsPen import BoundsPen


class ApertureDetector:
    """Detect and measure aperture openness in fonts."""
    
    def __init__(self, font_path: str):
        """Initialize detector with font path."""
        self.font_path = Path(font_path)
        self.ttfont = TTFont(str(self.font_path))
        self.glyph_set = self.ttfont.getGlyphSet()
        
        # Get x-height for normalization
        self.x_height = self._get_x_height()
    
    def _get_x_height(self) -> float:
        """Get x-height from font metrics."""
        # Try to get from OS/2 table
        if 'OS/2' in self.ttfont:
            os2 = self.ttfont['OS/2']
            if hasattr(os2, 'sxHeight') and os2.sxHeight > 0:
                return os2.sxHeight
        
        # Fallback: measure lowercase 'x'
        if 'x' in self.glyph_set:
            pen = BoundsPen(self.glyph_set)
            self.glyph_set['x'].draw(pen)
            bounds = pen.bounds
            if bounds:
                return bounds[3] - bounds[1]  # yMax - yMin
        
        # Ultimate fallback: estimate from em square
        return self.ttfont['head'].unitsPerEm * 0.5
    
    def extract_contours(self, glyph_name: str) -> List[List[Tuple[float, float]]]:
        """
        Extract contour points from a glyph.
        
        Returns:
            List of contours, each contour is list of (x, y) points
        """
        if glyph_name not in self.glyph_set:
            return []
        
        pen = RecordingPen()
        self.glyph_set[glyph_name].draw(pen)
        
        contours = []
        current_contour = []
        
        for op, args in pen.value:
            if op == 'moveTo':
                if current_contour:
                    contours.append(current_contour)
                current_contour = [args[0]]
            elif op == 'lineTo':
                current_contour.append(args[0])
            elif op == 'qCurveTo':
                # Quadratic bezier points
                for point in args:
                    current_contour.append(point)
            elif op == 'curveTo':
                # Cubic bezier points
                for point in args:
                    current_contour.append(point)
            elif op == 'closePath':
                if current_contour:
                    contours.append(current_contour)
                    current_contour = []
        
        if current_contour:
            contours.append(current_contour)
        
        return contours
    
    def is_closed_contour(self, contour: List[Tuple[float, float]]) -> bool:
        """Check if a contour is closed (first point == last point)."""
        if len(contour) < 2:
            return False
        
        first = contour[0]
        last = contour[-1]
        
        # Check if endpoints are very close (within 1 unit)
        distance = np.sqrt((first[0] - last[0])**2 + (first[1] - last[1])**2)
        return distance < 1.0
    
    def find_opening_width(self, contour: List[Tuple[float, float]]) -> Optional[float]:
        """
        Find the width of an opening in a contour.
        
        For open contours (like 'c'), measure distance between endpoints.
        """
        if self.is_closed_contour(contour):
            return None
        
        if len(contour) < 2:
            return None
        
        # Opening width = distance between first and last points
        first = np.array(contour[0])
        last = np.array(contour[-1])
        
        distance = np.linalg.norm(last - first)
        return float(distance)
    
    def measure_glyph_bounds(self, glyph_name: str) -> Optional[Tuple]:
        """Get glyph bounding box."""
        if glyph_name not in self.glyph_set:
            return None
        
        pen = BoundsPen(self.glyph_set)
        self.glyph_set[glyph_name].draw(pen)
        return pen.bounds
    
    def analyze_counter_opening(self, contours: List[List[Tuple[float, float]]]) -> Optional[float]:
        """
        Analyze counter opening by finding the narrowest gap.
        
        For glyphs like 'c' and 'e', even though contours are closed,
        there's a visual opening. We find this by:
        1. Finding the approximate opening region
        2. Measuring the gap width
        """
        if len(contours) < 2:
            # Single contour - check if it's open
            if len(contours) == 1:
                return self.find_opening_width(contours[0])
            return None
        
        # For glyphs with multiple contours (outer + inner)
        # Find the narrowest horizontal gap between contours
        outer_contour = contours[0]
        inner_contours = contours[1:]
        
        # Sample points and find minimum distance
        min_gaps = []
        
        for inner in inner_contours:
            # Sample along both contours
            outer_points = np.array(outer_contour)
            inner_points = np.array(inner)
            
            # Find minimum distances
            for inner_pt in inner_points:
                distances = np.linalg.norm(outer_points - inner_pt, axis=1)
                min_gaps.append(np.min(distances))
        
        if min_gaps:
            return float(np.min(min_gaps))
        
        return None
    
    def detect_aperture_via_convex_hull(self, contour: List[Tuple[float, float]]) -> Optional[float]:
        """
        Detect aperture by comparing contour to its convex hull.
        
        For glyphs like 'c', the convex hull will "close" the opening.
        The difference between hull and contour indicates the aperture.
        """
        if len(contour) < 3:
            return None
        
        from scipy.spatial import ConvexHull
        
        try:
            points = np.array(contour)
            
            # Compute convex hull
            hull = ConvexHull(points)
            hull_points = points[hull.vertices]
            
            # Find the maximum perpendicular distance from contour to hull
            # This represents the "depth" of the opening
            max_distances = []
            
            # For each hull edge, find farthest contour point
            for i in range(len(hull_points)):
                p1 = hull_points[i]
                p2 = hull_points[(i + 1) % len(hull_points)]
                
                # Compute distances from all contour points to this edge
                edge_vec = p2 - p1
                edge_length = np.linalg.norm(edge_vec)
                
                if edge_length < 1:
                    continue
                
                edge_normal = np.array([-edge_vec[1], edge_vec[0]]) / edge_length
                
                # Find points on the "inside" of this edge
                for pt in points:
                    vec_to_pt = pt - p1
                    distance = np.abs(np.dot(vec_to_pt, edge_normal))
                    
                    # Check if point is inside hull
                    if distance > 1:  # Threshold for significance
                        max_distances.append(distance)
            
            if max_distances:
                # The maximum indent represents the aperture depth
                aperture_depth = np.max(max_distances)
                
                # Estimate aperture width (heuristic: depth * 1.5)
                # This is an approximation since we can't easily measure the exact opening width
                aperture_width = aperture_depth * 1.5
                
                return float(aperture_width)
        
        except Exception as e:
            # Convex hull failed (degenerate case)
            return None
        
        return None
    
    def measure_aperture(self, glyph_name: str) -> Dict:
        """
        Measure aperture openness for a specific glyph.
        
        Enhanced version with multiple detection strategies:
        1. Open contours (simple case)
        2. Counter gaps (multi-contour glyphs like 'e', 'a')
        3. Convex hull analysis (single-contour glyphs like 'c', 's')
        
        Args:
            glyph_name: Character to analyze (e.g., 'c', 'e')
        
        Returns:
            {
                'opening_width': float,  # Raw distance in font units
                'normalized_score': float,  # Relative to x-height (0-1)
                'glyph': str,
                'has_aperture': bool,
                'detection_method': str  # Which method found the aperture
            }
        """
        contours = self.extract_contours(glyph_name)
        
        if not contours:
            return {
                'opening_width': 0.0,
                'normalized_score': 0.0,
                'glyph': glyph_name,
                'has_aperture': False,
                'error': 'Glyph not found'
            }
        
        opening_width = None
        detection_method = None
        
        # Method 1: Check for open contours (simple case)
        for contour in contours:
            width = self.find_opening_width(contour)
            if width is not None and width > 10:
                opening_width = width if opening_width is None else max(opening_width, width)
                detection_method = 'open_contour'
        
        # Method 2: Analyze counter gaps (for multi-contour glyphs like 'e', 'a')
        if opening_width is None or opening_width < 10:
            counter_gap = self.analyze_counter_opening(contours)
            if counter_gap is not None and counter_gap > 10:
                opening_width = counter_gap
                detection_method = 'counter_gap'
        
        # Method 3: Convex hull analysis (for closed glyphs like 'c', 's')
        if opening_width is None or opening_width < 10:
            # Try each contour
            for contour in contours:
                hull_aperture = self.detect_aperture_via_convex_hull(contour)
                if hull_aperture is not None and hull_aperture > 10:
                    opening_width = hull_aperture if opening_width is None else max(opening_width, hull_aperture)
                    detection_method = 'convex_hull'
        
        if opening_width is None or opening_width < 1:
            return {
                'opening_width': 0.0,
                'normalized_score': 0.0,
                'glyph': glyph_name,
                'has_aperture': False
            }
        
        # Normalize by x-height
        normalized = opening_width / self.x_height if self.x_height > 0 else 0.0
        
        return {
            'opening_width': float(opening_width),
            'normalized_score': float(normalized),
            'glyph': glyph_name,
            'has_aperture': True,
            'detection_method': detection_method
        }
    
    def analyze_font_aperture(
        self,
        glyphs: list = None
    ) -> Dict:
        """
        Analyze aperture openness across multiple glyphs.
        
        Args:
            glyphs: List of glyphs to analyze (default: ['c', 'e', 's', 'n', 'a'])
        
        Returns:
            {
                'mean_aperture': float,  # 0-1 scale
                'by_glyph': {...},
                'classification': str  # 'narrow', 'moderate', 'wide'
            }
        """
        if glyphs is None:
            glyphs = ['c', 'e', 's', 'n', 'a']
        
        results = {}
        aperture_scores = []
        
        for glyph in glyphs:
            try:
                result = self.measure_aperture(glyph)
                results[glyph] = result
                
                if result.get('has_aperture'):
                    aperture_scores.append(result['normalized_score'])
            except Exception as e:
                results[glyph] = {'error': str(e)}
        
        if not aperture_scores:
            return {
                'mean_aperture': 0.0,
                'by_glyph': results,
                'classification': 'unknown'
            }
        
        mean_aperture = np.mean(aperture_scores)
        
        # Classify aperture openness
        if mean_aperture < 0.4:
            classification = 'narrow'
        elif mean_aperture < 0.6:
            classification = 'moderate'
        else:
            classification = 'wide'
        
        return {
            'mean_aperture': float(mean_aperture),
            'by_glyph': results,
            'classification': classification
        }


# CLI for testing
if __name__ == '__main__':
    import sys
    
    if len(sys.argv) < 2:
        print("Usage: python aperture_detector.py <font.ttf>")
        sys.exit(1)
    
    font_path = sys.argv[1]
    
    print(f"\n{'='*70}")
    print(f"APERTURE ANALYSIS: {Path(font_path).name}")
    print(f"{'='*70}\n")
    
    detector = ApertureDetector(font_path)
    
    # Analyze
    results = detector.analyze_font_aperture()
    
    print(f"Overall Classification: {results['classification'].upper()}")
    print(f"Mean Aperture Score: {results['mean_aperture']:.3f}\n")
    
    print("Per-Glyph Results:")
    print(f"{'─'*70}")
    
    for glyph, data in results['by_glyph'].items():
        if 'error' in data:
            print(f"  {glyph}: {data['error']}")
        elif not data.get('has_aperture'):
            print(f"  {glyph}: No aperture detected")
        else:
            method = data.get('detection_method', 'unknown')
            print(f"  {glyph}: Score {data['normalized_score']:.3f} "
                  f"(width: {data['opening_width']:.0f} units) "
                  f"[{method}]")
    
    print(f"\n{'='*70}\n")
