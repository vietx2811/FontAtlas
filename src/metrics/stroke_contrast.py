"""
Stroke Contrast Analyzer
=========================

Measure stroke contrast ratio using distance transform and skeletonization.

This is a custom typography-specific metric that quantifies the variation
between thick and thin parts of character strokes.

Algorithm:
1. Render glyph as binary image
2. Apply distance transform to find stroke thickness at each point
3. Extract skeleton (medial axis) of the stroke
4. Sample thickness values along skeleton
5. Calculate max/min ratio

High contrast (>3.0): Modern serifs (Bodoni, Didot)
Moderate contrast (2.0-3.0): Transitional serifs (Garamond, Baskerville)
Low contrast (<1.5): Geometric sans (Helvetica, Roboto)
"""

from pathlib import Path
from typing import Dict, Tuple, Optional
import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage
from skimage.morphology import skeletonize
from fontTools.ttLib import TTFont


class StrokeContrastAnalyzer:
    """Analyze stroke contrast in fonts."""
    
    def __init__(self, font_path: str):
        """Initialize analyzer with font path."""
        self.font_path = Path(font_path)
        self.ttfont = TTFont(str(self.font_path))
    
    def render_glyph_binary(
        self,
        glyph: str,
        size: int = 256,
        padding: int = 32
    ) -> np.ndarray:
        """
        Render glyph as binary (black/white) image.
        
        Args:
            glyph: Character to render (e.g., 'o')
            size: Image size in pixels
            padding: Padding around glyph
        
        Returns:
            Binary numpy array (True = stroke, False = background)
        """
        # Create white image
        img = Image.new('L', (size, size), color=255)
        draw = ImageDraw.Draw(img)
        
        # Load font at appropriate size
        font_size = size - padding * 2
        try:
            pil_font = ImageFont.truetype(str(self.font_path), font_size)
        except Exception as e:
            raise ValueError(f"Failed to load font: {e}")
        
        # Get glyph bounding box
        bbox = draw.textbbox((0, 0), glyph, font=pil_font)
        text_width = bbox[2] - bbox[0]
        text_height = bbox[3] - bbox[1]
        
        # Center glyph
        x = (size - text_width) // 2 - bbox[0]
        y = (size - text_height) // 2 - bbox[1]
        
        # Draw glyph in black
        draw.text((x, y), glyph, font=pil_font, fill=0)
        
        # Convert to binary array (invert so stroke=True)
        binary = np.array(img) < 128
        
        return binary
    
    def measure_contrast_ratio(
        self,
        glyph: str = 'o',
        size: int = 256
    ) -> Dict:
        """
        Measure stroke contrast ratio for a specific glyph.
        
        Args:
            glyph: Character to analyze (default: 'o')
            size: Render size
        
        Returns:
            {
                'contrast_ratio': float,
                'max_thickness': float,
                'min_thickness': float,
                'mean_thickness': float,
                'glyph': str
            }
        """
        # 1. Render glyph
        binary_img = self.render_glyph_binary(glyph, size=size)
        
        # Check if glyph exists
        if not binary_img.any():
            return {
                'contrast_ratio': 1.0,
                'max_thickness': 0.0,
                'min_thickness': 0.0,
                'mean_thickness': 0.0,
                'glyph': glyph,
                'error': 'Glyph not found or empty'
            }
        
        # 2. Distance transform
        # Each point gets distance to nearest background pixel
        dist_transform = ndimage.distance_transform_edt(binary_img)
        
        # 3. Skeletonize to find medial axis
        skeleton = skeletonize(binary_img)
        
        # 4. Sample thickness at skeleton points
        # Distance transform gives radius, thickness = 2 * radius
        skeleton_points = np.argwhere(skeleton)
        
        if len(skeleton_points) == 0:
            return {
                'contrast_ratio': 1.0,
                'max_thickness': 0.0,
                'min_thickness': 0.0,
                'mean_thickness': 0.0,
                'glyph': glyph,
                'error': 'Skeletonization failed'
            }
        
        thicknesses = []
        for point in skeleton_points:
            y, x = point
            # Thickness = diameter = 2 * radius
            thickness = dist_transform[y, x] * 2.0
            thicknesses.append(thickness)
        
        thicknesses = np.array(thicknesses)
        
        # 5. Calculate statistics
        max_thickness = np.max(thicknesses)
        min_thickness = np.min(thicknesses[thicknesses > 0])  # Exclude zeros
        mean_thickness = np.mean(thicknesses)
        
        # Avoid division by zero
        if min_thickness < 0.1:
            min_thickness = 0.1
        
        contrast_ratio = max_thickness / min_thickness
        
        return {
            'contrast_ratio': float(contrast_ratio),
            'max_thickness': float(max_thickness),
            'min_thickness': float(min_thickness),
            'mean_thickness': float(mean_thickness),
            'glyph': glyph
        }
    
    def analyze_font_contrast(
        self,
        glyphs: list = None
    ) -> Dict:
        """
        Analyze stroke contrast across multiple glyphs.
        
        Args:
            glyphs: List of glyphs to analyze (default: ['o', 'O', 'n'])
        
        Returns:
            {
                'mean_contrast': float,
                'by_glyph': {
                    'o': {...},
                    'O': {...}
                },
                'classification': str  # 'low', 'moderate', 'high'
            }
        """
        if glyphs is None:
            glyphs = ['o', 'O', 'n']
        
        results = {}
        contrast_ratios = []
        
        for glyph in glyphs:
            try:
                result = self.measure_contrast_ratio(glyph)
                results[glyph] = result
                
                if 'error' not in result:
                    contrast_ratios.append(result['contrast_ratio'])
            except Exception as e:
                results[glyph] = {'error': str(e)}
        
        if not contrast_ratios:
            return {
                'mean_contrast': 1.0,
                'by_glyph': results,
                'classification': 'unknown'
            }
        
        mean_contrast = np.mean(contrast_ratios)
        
        # Classify contrast level
        if mean_contrast < 1.5:
            classification = 'low'
        elif mean_contrast < 2.5:
            classification = 'moderate'
        else:
            classification = 'high'
        
        return {
            'mean_contrast': float(mean_contrast),
            'by_glyph': results,
            'classification': classification
        }
    
    def save_debug_images(
        self,
        glyph: str = 'o',
        output_dir: str = 'debug_stroke'
    ) -> None:
        """
        Save debug images showing intermediate steps.
        
        Useful for visualizing the algorithm.
        """
        from pathlib import Path
        import matplotlib.pyplot as plt
        
        output_path = Path(output_dir)
        output_path.mkdir(exist_ok=True)
        
        # Render
        binary = self.render_glyph_binary(glyph)
        dist = ndimage.distance_transform_edt(binary)
        skeleton = skeletonize(binary)
        
        # Create figure
        fig, axes = plt.subplots(1, 4, figsize=(16, 4))
        
        axes[0].imshow(binary, cmap='gray')
        axes[0].set_title('1. Binary Glyph')
        axes[0].axis('off')
        
        axes[1].imshow(dist, cmap='viridis')
        axes[1].set_title('2. Distance Transform')
        axes[1].axis('off')
        
        axes[2].imshow(skeleton, cmap='gray')
        axes[2].set_title('3. Skeleton')
        axes[2].axis('off')
        
        # Overlay skeleton on distance transform
        axes[3].imshow(dist, cmap='viridis', alpha=0.7)
        axes[3].imshow(skeleton, cmap='Reds', alpha=0.5)
        axes[3].set_title('4. Skeleton + Distance')
        axes[3].axis('off')
        
        plt.tight_layout()
        
        font_name = self.font_path.stem
        output_file = output_path / f"{font_name}_{glyph}_debug.png"
        plt.savefig(output_file, dpi=150, bbox_inches='tight')
        plt.close()
        
        print(f"Debug images saved to: {output_file}")


# CLI for testing
if __name__ == '__main__':
    import sys
    import json
    
    if len(sys.argv) < 2:
        print("Usage: python stroke_contrast.py <font.ttf> [--debug]")
        sys.exit(1)
    
    font_path = sys.argv[1]
    debug = '--debug' in sys.argv
    
    print(f"\n{'='*70}")
    print(f"STROKE CONTRAST ANALYSIS: {Path(font_path).name}")
    print(f"{'='*70}\n")
    
    analyzer = StrokeContrastAnalyzer(font_path)
    
    # Analyze
    results = analyzer.analyze_font_contrast()
    
    print(f"Overall Classification: {results['classification'].upper()}")
    print(f"Mean Contrast Ratio: {results['mean_contrast']:.3f}\n")
    
    print("Per-Glyph Results:")
    print(f"{'─'*70}")
    
    for glyph, data in results['by_glyph'].items():
        if 'error' in data:
            print(f"  {glyph}: {data['error']}")
        else:
            print(f"  {glyph}: Ratio {data['contrast_ratio']:.3f} "
                  f"(max: {data['max_thickness']:.1f}px, "
                  f"min: {data['min_thickness']:.1f}px)")
    
    print(f"\n{'='*70}\n")
    
    # Save debug images if requested
    if debug:
        analyzer.save_debug_images('o')
        print("Debug images saved.\n")
