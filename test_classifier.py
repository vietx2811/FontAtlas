#!/usr/bin/env python3
"""
Test Enhanced Font Classifier with Google Fonts
================================================

Downloads and tests representative fonts to validate the new taxonomy system.
"""

import sys
import json
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent.parent / 'src'))

from parsers.font_parser import FontParser
from metrics.font_classifier import FontClassifier

# Test fonts (you can place these in a test_fonts directory)
TEST_FONTS = {
    'Roboto-Regular.ttf': {
        'expected_class': 'sans',
        'expected_subtype': 'neo-grotesque or grotesque',
        'expected_tags': ['low-contrast', 'clean', 'modern']
    },
    'PlayfairDisplay-Regular.ttf': {
        'expected_class': 'serif',
        'expected_subtype': 'didone or transitional',
        'expected_tags': ['high-contrast', 'elegant', 'luxury']
    },
    'Pacifico-Regular.ttf': {
        'expected_class': 'script',
        'expected_subtype': 'brush or casual',
        'expected_tags': ['warm']
    },
    'RobotoMono-Regular.ttf': {
        'expected_class': 'monospace',
        'expected_subtype': None,
        'expected_tags': ['fixed-width', 'tech']
    },
    'Montserrat-Regular.ttf': {
        'expected_class': 'sans',
        'expected_subtype': 'geometric',
        'expected_tags': ['tall-x-height', 'modern']
    }
}

def test_font_classification(font_path: str, expected: dict):
    """Test a single font and display results."""
    print(f"\n{'='*70}")
    print(f"Testing: {Path(font_path).name}")
    print(f"{'='*70}")
    
    try:
        # Parse font
        parser = FontParser(font_path)
        profile = parser.parse()
        
        # Classify with enhanced taxonomy
        result = FontClassifier.classify_enhanced(profile)
        
        # Display results
        print(f"\n📊 CLASSIFICATION RESULTS:")
        print(f"  Primary Class: {result['primary_class'].upper()}")
        
        if result['serif_subtype']:
            print(f"  Serif Subtype: {result['serif_subtype']}")
        if result['sans_subtype']:
            print(f"  Sans Subtype: {result['sans_subtype']}")
        if result['script_subtype']:
            print(f"  Script Subtype: {result['script_subtype']}")
        
        print(f"\n🏷️  SECONDARY TAGS:")
        if result['secondary_tags']:
            for tag in result['secondary_tags']:
                print(f"    • {tag}")
        else:
            print(f"    (none)")
        
        print(f"\n💼 SUITABLE ROLES:")
        if result['suitable_roles']:
            for role in result['suitable_roles']:
                print(f"    • {role}")
        else:
            print(f"    (none)")
        
        # Validation
        print(f"\n✅ VALIDATION:")
        primary_match = result['primary_class'] == expected['expected_class']
        print(f"  Primary Class: {'✓' if primary_match else '✗'} (expected: {expected['expected_class']})")
        
        if expected['expected_subtype']:
            subtype = result.get('serif_subtype') or result.get('sans_subtype') or result.get('script_subtype')
            print(f"  Subtype: {subtype} (expected: {expected['expected_subtype']})")
        
        if expected['expected_tags']:
            found_tags = set(result['secondary_tags']) & set(expected['expected_tags'])
            print(f"  Expected Tags Found: {len(found_tags)}/{len(expected['expected_tags'])} - {list(found_tags)}")
        
        # JSON output for debugging
        print(f"\n📋 FULL JSON OUTPUT:")
        print(json.dumps(result, indent=2))
        
        return True
        
    except Exception as e:
        print(f"\n❌ ERROR: {e}")
        import traceback
        traceback.print_exc()
        return False

def main():
    """Run classification tests."""
    print(f"\n{'#'*70}")
    print(f"# ENHANCED FONT CLASSIFIER TEST SUITE")
    print(f"{'#'*70}")
    
    # Check for test fonts directory
    test_fonts_dir = Path(__file__).parent / 'test_fonts'
    
    if not test_fonts_dir.exists():
        print(f"\n⚠️  Test fonts directory not found: {test_fonts_dir}")
        print(f"\nTo run tests:")
        print(f"  1. Create directory: mkdir -p {test_fonts_dir}")
        print(f"  2. Download test fonts:")
        print(f"     - Roboto-Regular.ttf (neo-grotesque sans)")
        print(f"     - PlayfairDisplay-Regular.ttf (high-contrast serif)")
        print(f"     - Pacifico-Regular.ttf (script)")
        print(f"     - RobotoMono-Regular.ttf (monospace)")
        print(f"     - Montserrat-Regular.ttf (geometric sans)")
        print(f"\n  Or provide your own font paths as arguments:")
        print(f"     python test_classifier.py /path/to/font1.ttf /path/to/font2.ttf")
        
        # Check if fonts provided as arguments
        if len(sys.argv) > 1:
            print(f"\n\nTesting provided fonts...\n")
            for font_path in sys.argv[1:]:
                test_font_classification(font_path, {
                    'expected_class': 'unknown',
                    'expected_subtype': None,
                    'expected_tags': []
                })
        return
    
    # Run tests
    results = []
    for font_file, expected in TEST_FONTS.items():
        font_path = test_fonts_dir / font_file
        if font_path.exists():
            success = test_font_classification(str(font_path), expected)
            results.append((font_file, success))
        else:
            print(f"\n⚠️  Font not found: {font_path}")
            results.append((font_file, False))
    
    # Summary
    print(f"\n\n{'='*70}")
    print(f"TEST SUMMARY")
    print(f"{'='*70}")
    successful = sum(1 for _, success in results if success)
    print(f"\nTests Passed: {successful}/{len(results)}")
    
    for font_file, success in results:
        status = "✓" if success else "✗"
        print(f"  {status} {font_file}")
    
    print(f"\n{'='*70}\n")

if __name__ == '__main__':
    main()
