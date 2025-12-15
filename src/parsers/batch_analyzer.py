"""
Batch Font Analyzer
===================

Analyze multiple fonts and compare metrics.
"""

from pathlib import Path
import json
import sys

# Add src to path
sys.path.insert(0, 'src')
from parsers.font_parser import FontParser


def analyze_font_batch(fonts_dir: str = 'fonts/samples'):
    """
    Analyze all fonts in a directory.
    
    Args:
        fonts_dir: Directory containing TTF fonts
    """
    fonts_path = Path(fonts_dir)
    font_files = list(fonts_path.glob('*.ttf'))
    
    if not font_files:
        print(f"No TTF fonts found in {fonts_dir}")
        return
    
    print(f"\n{'='*70}")
    print(f"FontAtlas Phase 1A - Batch Analysis")
    print(f"{'='*70}\n")
    print(f"Analyzing {len(font_files)} fonts...\n")
    
    results = []
    
    for font_file in sorted(font_files):
        try:
            print(f"{'─'*70}")
            parser = FontParser(str(font_file))
            
            # Get metrics
            metrics = parser.get_basic_metrics()
            coverage = parser.get_unicode_coverage()
            
            # Display
            print(f"📘 {parser.font_name}")
            print(f"   File: {font_file.name}")
            print(f"\n   📏 Metrics:")
            print(f"      x-height ratio: {metrics.get('x_height_ratio', 'N/A'):.4f}" if metrics.get('x_height_ratio') else "      x-height ratio: N/A")
            print(f"      Width variance: {metrics.get('width_variance', 'N/A'):.4f}" if metrics.get('width_variance') else "      Width variance: N/A")
            print(f"      Width mean: {metrics.get('width_mean', 'N/A'):.4f}" if metrics.get('width_mean') else "      Width mean: N/A")
            
            print(f"\n   🌍 Unicode Coverage:")
            print(f"      Total glyphs: {coverage['total_glyphs']}")
            print(f"      Vietnamese: {'✓' if coverage['supports_vietnamese'] else '✗'}")
            print(f"      Latin Extended: {'✓' if coverage['supports_latin_extended'] else '✗'}")
            
            # Coverage percentages
            for range_name, pct in coverage['unicode_ranges'].items():
                if pct > 0:
                    print(f"      {range_name}: {pct*100:.1f}%")
            
            print()
            
            # Store result
            results.append({
                'font_name': parser.font_name,
                'file': font_file.name,
                'metrics': metrics,
                'coverage': coverage
            })
            
        except Exception as e:
            print(f"✗ Error analyzing {font_file.name}: {e}\n")
    
    # Save batch results
    output_path = fonts_path / 'batch_analysis.json'
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(results, f, indent=2, ensure_ascii=False)
    
    print(f"{'='*70}")
    print(f"✓ Batch analysis complete!")
    print(f"✓ Results saved to {output_path}")
    print(f"{'='*70}\n")
    
    # Comparison summary
    print("📊 Comparison Summary:\n")
    
    # Sort by x-height ratio
    valid_fonts = [r for r in results if r['metrics'].get('x_height_ratio')]
    if valid_fonts:
        by_xheight = sorted(valid_fonts, key=lambda x: x['metrics']['x_height_ratio'])
        
        print("   X-Height Ratio (Low → High):")
        for font in by_xheight:
            ratio = font['metrics']['x_height_ratio']
            print(f"      {font['font_name']:30} {ratio:.4f}")
        
        print()
    
    # Sort by width variance
    by_width_var = sorted(valid_fonts, key=lambda x: x['metrics'].get('width_variance', 0))
    
    print("   Width Variance (Low → High):")
    for font in by_width_var:
        var = font['metrics'].get('width_variance', 0)
        print(f"      {font['font_name']:30} {var:.4f}")


if __name__ == '__main__':
    analyze_font_batch()
