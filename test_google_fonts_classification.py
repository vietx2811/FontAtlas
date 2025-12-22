#!/usr/bin/env python3
"""
Google Fonts Classification Test
=================================
Download sample fonts from Google Fonts and test classification algorithm accuracy.

Usage:
    python3 test_google_fonts_classification.py
"""

import os
import sys
import json
import urllib.request
import tempfile
from pathlib import Path

# Add FontAtlas src to path
SCRIPT_DIR = Path(__file__).parent
sys.path.insert(0, str(SCRIPT_DIR / "src"))

from parsers.font_parser import FontParser

# Sample fonts to test - manually curated list with known categories
# Format: (font_family, expected_category, variant)
# Note: Display is a style/usage, not a primary class.
# Fonts are classified by anatomy: serif, sans, script, monospace, symbol
GOOGLE_FONTS_TEST_SET = [
    # === SANS SERIF ===
    ("Roboto", "sans", "regular"),
    ("Open Sans", "sans", "regular"),
    ("Lato", "sans", "regular"),
    ("Montserrat", "sans", "regular"),
    ("Oswald", "sans", "regular"),
    ("Poppins", "sans", "regular"),
    ("Inter", "sans", "regular"),
    ("Nunito", "sans", "regular"),
    ("Ubuntu", "sans", "regular"),
    ("Raleway", "sans", "regular"),
    ("Bebas Neue", "sans", "regular"),  # Display sans
    ("Righteous", "sans", "regular"),    # Display sans
    ("Passion One", "sans", "regular"),  # Display sans
    
    # === SERIF ===
    ("Playfair Display", "serif", "regular"),
    ("Merriweather", "serif", "regular"),
    ("Lora", "serif", "regular"),
    ("PT Serif", "serif", "regular"),
    ("Libre Baskerville", "serif", "regular"),
    ("Crimson Text", "serif", "regular"),
    ("Cormorant Garamond", "serif", "regular"),
    ("EB Garamond", "serif", "regular"),
    ("Source Serif Pro", "serif", "regular"),
    ("Bitter", "serif", "regular"),
    ("Abril Fatface", "serif", "regular"),    # Display serif
    ("Alfa Slab One", "serif", "regular"),    # Display serif (slab)
    ("Lobster", "script", "regular"),         # Display script (asymmetric)
    
    # === SCRIPT (true handwriting/cursive) ===
    ("Dancing Script", "script", "regular"),
    ("Pacifico", "script", "regular"),
    ("Great Vibes", "script", "regular"),
    ("Satisfy", "script", "regular"),
    ("Caveat", "script", "regular"),
    ("Kaushan Script", "script", "regular"),
    ("Sacramento", "script", "regular"),
    
    # === MONOSPACE ===
    ("Roboto Mono", "monospace", "regular"),
    ("Source Code Pro", "monospace", "regular"),
    ("Fira Code", "monospace", "regular"),
    ("JetBrains Mono", "monospace", "regular"),
    ("IBM Plex Mono", "monospace", "regular"),
]




def get_google_font_url(family: str, variant: str = "regular") -> str:
    """Get download URL for a Google Font."""
    # Format family name for URL
    family_url = family.replace(" ", "+")
    
    # Use CSS2 API to get font files
    css_url = f"https://fonts.googleapis.com/css2?family={family_url}&display=swap"
    
    try:
        req = urllib.request.Request(css_url, headers={
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        })
        with urllib.request.urlopen(req, timeout=10) as response:
            css_content = response.read().decode('utf-8')
        
        # Extract TTF or WOFF2 URL from CSS
        import re
        # Look for url() in CSS
        urls = re.findall(r'url\((https://[^)]+\.(?:ttf|woff2))\)', css_content)
        if urls:
            return urls[0]
    except Exception as e:
        print(f"  Warning: Could not get URL for {family}: {e}", file=sys.stderr)
    
    return None


def download_font(url: str, output_path: Path) -> bool:
    """Download font file."""
    try:
        req = urllib.request.Request(url, headers={
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        })
        with urllib.request.urlopen(req, timeout=30) as response:
            content = response.read()
        
        output_path.write_bytes(content)
        return True
    except Exception as e:
        print(f"  Download failed: {e}", file=sys.stderr)
        return False


def test_classification(font_path: Path, expected_category: str) -> dict:
    """Test font classification and return results."""
    try:
        from font_classifier import FontClassifier
    except ImportError:
        # Try from sidecar location
        sys.path.insert(0, str(SCRIPT_DIR / "UX" / "python-sidecar"))
        from font_classifier import FontClassifier
    
    try:
        parser = FontParser(str(font_path))
        profile = parser.extract_full_profile()
        
        result = FontClassifier.classify_enhanced(profile)
        predicted = result.get('primary_class', 'unknown')
        
        # Get detection details
        serif_detection = profile.get('serif_detection', {})
        anatomy = profile.get('anatomy_metrics', {})
        
        return {
            'success': True,
            'predicted': predicted,
            'expected': expected_category,
            'match': predicted == expected_category,
            'command_count': serif_detection.get('command_count', 0),
            'curvature': anatomy.get('curvature_ratio', 0),
            'has_serifs': serif_detection.get('has_serifs'),
        }
    except Exception as e:
        return {
            'success': False,
            'error': str(e),
            'expected': expected_category,
        }


def run_tests():
    """Run classification tests on Google Fonts samples."""
    print("=" * 80)
    print("Google Fonts Classification Test")
    print("=" * 80)
    print()
    
    # Create temp directory for downloads
    temp_dir = Path(tempfile.mkdtemp(prefix="gfonts_test_"))
    print(f"Temp directory: {temp_dir}")
    print()
    
    results = []
    
    for family, expected, variant in GOOGLE_FONTS_TEST_SET:
        print(f"Testing: {family} (expected: {expected})")
        
        # Get font URL
        url = get_google_font_url(family, variant)
        if not url:
            print(f"  ✗ Could not get download URL")
            results.append({
                'family': family,
                'success': False,
                'error': 'No download URL'
            })
            continue
        
        # Download font
        ext = ".woff2" if url.endswith(".woff2") else ".ttf"
        font_path = temp_dir / f"{family.replace(' ', '_')}{ext}"
        
        if not download_font(url, font_path):
            results.append({
                'family': family,
                'success': False,
                'error': 'Download failed'
            })
            continue
        
        # Test classification
        test_result = test_classification(font_path, expected)
        test_result['family'] = family
        results.append(test_result)
        
        if test_result.get('success'):
            match_symbol = "✓" if test_result['match'] else "✗"
            print(f"  {match_symbol} Predicted: {test_result['predicted']} "
                  f"(cmd={test_result['command_count']:.0f}, curv={test_result['curvature']:.2f})")
        else:
            print(f"  ✗ Error: {test_result.get('error', 'Unknown')}")
    
    # Summary
    print()
    print("=" * 80)
    print("SUMMARY")
    print("=" * 80)
    
    successful = [r for r in results if r.get('success')]
    correct = [r for r in successful if r.get('match')]
    
    print(f"Total fonts tested: {len(results)}")
    print(f"Successfully analyzed: {len(successful)}")
    print(f"Correct classifications: {len(correct)}")
    
    if successful:
        accuracy = len(correct) / len(successful) * 100
        print(f"Accuracy: {accuracy:.1f}%")
    
    # Show misclassifications
    misclassified = [r for r in successful if not r.get('match')]
    if misclassified:
        print()
        print("MISCLASSIFICATIONS:")
        for r in misclassified:
            print(f"  {r['family']}: expected={r['expected']}, got={r['predicted']}")
    
    # Cleanup
    import shutil
    shutil.rmtree(temp_dir, ignore_errors=True)
    
    return results


if __name__ == "__main__":
    run_tests()
