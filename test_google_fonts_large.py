#!/usr/bin/env python3
"""
Large Scale Google Fonts Classification Test
=============================================
Test classification on 80+ fonts from Google Fonts.
"""

import os
import sys
import json
import urllib.request
import tempfile
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
sys.path.insert(0, str(SCRIPT_DIR / "src"))

from parsers.font_parser import FontParser

# Expanded test set - 80+ fonts
GOOGLE_FONTS_TEST_SET = [
    # === SANS SERIF (25 fonts) ===
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
    ("Bebas Neue", "sans", "regular"),
    ("Righteous", "sans", "regular"),
    ("Passion One", "sans", "regular"),
    ("Work Sans", "sans", "regular"),
    ("Quicksand", "sans", "regular"),
    ("Rubik", "sans", "regular"),
    ("Nunito Sans", "sans", "regular"),
    ("Karla", "sans", "regular"),
    ("Barlow", "sans", "regular"),
    ("Cabin", "sans", "regular"),
    ("Archivo", "sans", "regular"),
    ("DM Sans", "sans", "regular"),
    ("Manrope", "sans", "regular"),
    ("Outfit", "sans", "regular"),
    ("Lexend", "sans", "regular"),
    
    # === SERIF (20 fonts) ===
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
    ("Abril Fatface", "serif", "regular"),
    ("Alfa Slab One", "serif", "regular"),
    ("Noto Serif", "serif", "regular"),
    ("Vollkorn", "serif", "regular"),
    ("Frank Ruhl Libre", "serif", "regular"),
    ("Cardo", "serif", "regular"),
    ("Spectral", "serif", "regular"),
    ("Rokkitt", "serif", "regular"),
    ("Josefin Slab", "serif", "regular"),
    ("Zilla Slab", "serif", "regular"),
    
    # === SCRIPT/HANDWRITING (20 fonts) ===
    ("Dancing Script", "script", "regular"),
    ("Pacifico", "script", "regular"),
    ("Great Vibes", "script", "regular"),
    ("Satisfy", "script", "regular"),
    ("Caveat", "script", "regular"),
    ("Kaushan Script", "script", "regular"),
    ("Sacramento", "script", "regular"),
    ("Lobster", "script", "regular"),
    ("Courgette", "script", "regular"),
    ("Cookie", "script", "regular"),
    ("Allura", "script", "regular"),
    ("Alex Brush", "script", "regular"),
    ("Tangerine", "script", "regular"),
    ("Pinyon Script", "script", "regular"),
    ("Mr Dafoe", "script", "regular"),
    ("Parisienne", "script", "regular"),
    ("Yellowtail", "script", "regular"),
    ("Lobster Two", "script", "regular"),
    ("Indie Flower", "script", "regular"),
    ("Shadows Into Light", "script", "regular"),
    
    # === MONOSPACE (10 fonts) ===
    ("Roboto Mono", "monospace", "regular"),
    ("Source Code Pro", "monospace", "regular"),
    ("Fira Code", "monospace", "regular"),
    ("JetBrains Mono", "monospace", "regular"),
    ("IBM Plex Mono", "monospace", "regular"),
    ("Inconsolata", "monospace", "regular"),
    ("Ubuntu Mono", "monospace", "regular"),
    ("Space Mono", "monospace", "regular"),
    ("Cousine", "monospace", "regular"),
    ("Anonymous Pro", "monospace", "regular"),
]


def get_google_font_url(family: str) -> str:
    family_url = family.replace(" ", "+")
    css_url = f"https://fonts.googleapis.com/css2?family={family_url}&display=swap"
    
    try:
        import re
        req = urllib.request.Request(css_url, headers={
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        })
        with urllib.request.urlopen(req, timeout=10) as response:
            css_content = response.read().decode('utf-8')
        
        urls = re.findall(r'url\((https://[^)]+\.(?:ttf|woff2))\)', css_content)
        if urls:
            return urls[0]
    except Exception as e:
        pass
    
    return None


def download_font(url: str, output_path: Path) -> bool:
    try:
        req = urllib.request.Request(url, headers={
            "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
        })
        with urllib.request.urlopen(req, timeout=30) as response:
            content = response.read()
        
        output_path.write_bytes(content)
        return True
    except Exception:
        return False


def test_classification(font_path: Path, expected_category: str) -> dict:
    try:
        from font_classifier import FontClassifier
    except ImportError:
        sys.path.insert(0, str(SCRIPT_DIR / "UX" / "python-sidecar"))
        from font_classifier import FontClassifier
    
    try:
        parser = FontParser(str(font_path))
        profile = parser.extract_full_profile()
        
        result = FontClassifier.classify_enhanced(profile)
        predicted = result.get('primary_class', 'unknown')
        
        serif_detection = profile.get('serif_detection', {})
        symmetry_detection = profile.get('symmetry_detection', {})
        anatomy = profile.get('anatomy_metrics', {})
        
        return {
            'success': True,
            'predicted': predicted,
            'expected': expected_category,
            'match': predicted == expected_category,
            'command_count': serif_detection.get('command_count', 0),
            'curvature': anatomy.get('curvature_ratio', 0),
            'symmetry': symmetry_detection.get('symmetry_score', 0),
        }
    except Exception as e:
        return {
            'success': False,
            'error': str(e),
            'expected': expected_category,
        }


def run_tests():
    print("=" * 80)
    print("Large Scale Google Fonts Classification Test")
    print("=" * 80)
    print(f"Testing {len(GOOGLE_FONTS_TEST_SET)} fonts")
    print()
    
    temp_dir = Path(tempfile.mkdtemp(prefix="gfonts_large_"))
    
    results = []
    categories = {'sans': [], 'serif': [], 'script': [], 'monospace': []}
    
    for family, expected, variant in GOOGLE_FONTS_TEST_SET:
        print(f"Testing: {family:<25}", end="", flush=True)
        
        url = get_google_font_url(family)
        if not url:
            print("✗ No URL")
            results.append({'family': family, 'success': False, 'error': 'No URL'})
            continue
        
        ext = ".woff2" if url.endswith(".woff2") else ".ttf"
        font_path = temp_dir / f"{family.replace(' ', '_')}{ext}"
        
        if not download_font(url, font_path):
            print("✗ Download failed")
            results.append({'family': family, 'success': False, 'error': 'Download failed'})
            continue
        
        test_result = test_classification(font_path, expected)
        test_result['family'] = family
        results.append(test_result)
        
        if test_result.get('success'):
            match_symbol = "✓" if test_result['match'] else "✗"
            print(f"{match_symbol} {expected} → {test_result['predicted']:<10} "
                  f"(sym={test_result['symmetry']:.2f})")
            categories[expected].append(test_result)
        else:
            print(f"✗ Error: {test_result.get('error', 'Unknown')[:30]}")
    
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
        print(f"Overall Accuracy: {accuracy:.1f}%")
    
    print()
    print("BY CATEGORY:")
    for cat in ['sans', 'serif', 'script', 'monospace']:
        cat_results = categories[cat]
        if cat_results:
            cat_correct = sum(1 for r in cat_results if r.get('match'))
            cat_acc = cat_correct / len(cat_results) * 100
            print(f"  {cat:10}: {cat_correct}/{len(cat_results)} ({cat_acc:.0f}%)")
    
    # Show misclassifications
    misclassified = [r for r in successful if not r.get('match')]
    if misclassified:
        print()
        print("MISCLASSIFICATIONS:")
        for r in misclassified:
            print(f"  {r['family']}: expected={r['expected']}, got={r['predicted']} "
                  f"(sym={r.get('symmetry', 0):.2f})")
    
    # Cleanup
    import shutil
    shutil.rmtree(temp_dir, ignore_errors=True)
    
    return results


if __name__ == "__main__":
    run_tests()
