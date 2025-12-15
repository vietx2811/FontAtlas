"""
Metadata Viewer & Comparison Tool
==================================

Visualize and compare enriched font profiles.
"""

import json
from pathlib import Path
from typing import List, Dict
import sys

sys.path.insert(0, 'src')


def view_enriched_profile(profile_path: str):
    """Display enriched profile in readable format."""
    with open(profile_path, 'r') as f:
        profile = json.load(f)
    
    print(f"\n{'='*70}")
    print(f"📘 {profile['metadata']['font_name']}")
    print(f"{'='*70}\n")
    
    # Basic Metrics
    metrics = profile['basic_metrics']
    print("📏 BASIC METRICS (OSS-Supported)")
    print(f"   x-height ratio: {metrics.get('x_height_ratio', 'N/A'):.4f}" if metrics.get('x_height_ratio') else "   x-height ratio: N/A")
    print(f"   Width variance: {metrics.get('width_variance', 'N/A'):.4f}" if metrics.get('width_variance') else "   Width variance: N/A")
    print(f"   Width mean: {metrics.get('width_mean', 'N/A'):.4f}" if metrics.get('width_mean') else "   Width mean: N/A")
    print()
    
    # Unicode Coverage
    coverage = profile['unicode_coverage']
    print("🌍 UNICODE COVERAGE")
    print(f"   Total glyphs: {coverage['total_glyphs']}")
    print(f"   Vietnamese: {'✓' if coverage['supports_vietnamese'] else '✗'}")
    print(f"   Latin Extended: {'✓' if coverage['supports_latin_extended'] else '✗'}")
    print()
    
    # Metadata Layer
    metadata_layer = profile.get('metadata_layer', {})
    
    if 'google_fonts' in metadata_layer:
        gf = metadata_layer['google_fonts']
        print("📦 GOOGLE FONTS METADATA")
        print(f"   Family: {gf.get('family')}")
        print(f"   Category: {gf.get('category')}")
        print(f"   Designers: {', '.join(gf.get('designers', []))}")
        print(f"   Popularity rank: #{gf.get('popularity_rank', 'N/A')}")
        print(f"   Date added: {gf.get('date_added')}")
        print(f"   Subsets: {len(gf.get('subsets', []))} ({', '.join(gf.get('subsets', [])[:5])}...)")
        print()
    
    if 'font_internal' in metadata_layer:
        internal = metadata_layer['font_internal']
        print("🔧 INTERNAL FONT METADATA")
        print(f"   Full name: {internal.get('full_name')}")
        print(f"   Version: {internal.get('version')}")
        print(f"   Designer: {internal.get('designer', 'N/A')}")
        print(f"   Copyright: {internal.get('copyright', 'N/A')[:60]}...")
        print(f"   License: {internal.get('license_url', 'N/A')}")
        print()


def compare_fonts(profile_paths: List[str]):
    """Compare multiple enriched profiles."""
    profiles = []
    for path in profile_paths:
        with open(path, 'r') as f:
            profiles.append(json.load(f))
    
    print(f"\n{'='*70}")
    print(f"FONT COMPARISON ({len(profiles)} fonts)")
    print(f"{'='*70}\n")
    
    # Table header
    print(f"{'Font':25} {'Category':15} {'x-height':10} {'Width var':10} {'Rank':6}")
    print(f"{'-'*70}")
    
    for p in profiles:
        name = p['metadata']['font_name'][:24]
        
        category = 'N/A'
        rank = 'N/A'
        if 'google_fonts' in p.get('metadata_layer', {}):
            gf = p['metadata_layer']['google_fonts']
            category = gf.get('category', 'N/A')[:14]
            rank = f"#{gf.get('popularity_rank', 'N/A')}"
        
        x_height = p['basic_metrics'].get('x_height_ratio', 0)
        width_var = p['basic_metrics'].get('width_variance', 0)
        
        print(f"{name:25} {category:15} {x_height:10.4f} {width_var:10.4f} {rank:6}")
    
    print()


def main():
    """CLI for metadata viewer."""
    import argparse
    
    parser = argparse.ArgumentParser(description='View enriched font profiles')
    parser.add_argument('files', nargs='*', help='Enriched JSON files to view/compare')
    parser.add_argument('--all', action='store_true', help='View all in fonts/samples/')
    
    args = parser.parse_args()
    
    if args.all or not args.files:
        # View all enriched profiles
        samples_dir = Path('fonts/samples')
        enriched_files = list(samples_dir.glob('*.enriched.json'))
        
        if not enriched_files:
            print("No enriched profiles found. Run metadata_fetcher.py first.")
            return
        
        # Show comparison table
        compare_fonts([str(f) for f in enriched_files])
        
        # Show detailed view for first font
        print("\nDetailed view of first font:")
        view_enriched_profile(str(enriched_files[0]))
    
    else:
        # View specific files
        if len(args.files) == 1:
            view_enriched_profile(args.files[0])
        else:
            compare_fonts(args.files)


if __name__ == '__main__':
    main()
