"""
Google Fonts API Metadata Crawler
==================================

Fetches font metadata from Google Fonts API (no API key needed for public data).

Metadata enrichment per FontAtlas spec:
- Designer, foundry, category
- Popularity, trending
- Release year (if available)
- Variants available

CRITICAL: Metadata is stored SEPARATELY from geometry metrics.
This maintains epistemological separation per spec.
"""

import requests
import json
from typing import Dict, List, Optional
from pathlib import Path
import time


class GoogleFontsMetadata:
    """Fetch and manage Google Fonts metadata."""
    
    API_URL = "https://www.googleapis.com/webfonts/v1/webfonts"
    
    def __init__(self, api_key: Optional[str] = None, cache_dir: str = 'data/metadata'):
        """
        Initialize metadata fetcher.
        
        Args:
            api_key: Google API key (optional - can use without for basic data)
            cache_dir: Directory to cache API responses
        """
        self.api_key = api_key
        self.cache_dir = Path(cache_dir)
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        self.fonts_data = None
    
    def fetch_all_fonts(self, force_refresh: bool = False) -> Dict:
        """
        Fetch complete Google Fonts catalog.
        
        Args:
            force_refresh: Force API call instead of using cache
        
        Returns:
            Dict with 'items' list of font metadata
        """
        cache_file = self.cache_dir / 'google_fonts_catalog.json'
        
        # Check cache first
        if not force_refresh and cache_file.exists():
            print("✓ Using cached Google Fonts catalog")
            with open(cache_file, 'r', encoding='utf-8') as f:
                self.fonts_data = json.load(f)
            return self.fonts_data
        
        # Fetch from API
        print("Fetching Google Fonts catalog from API...")
        
        params = {}
        if self.api_key:
            params['key'] = self.api_key
        else:
            # Can still fetch basic data without API key (rate limited)
            params['sort'] = 'popularity'
        
        try:
            response = requests.get(self.API_URL, params=params, timeout=30)
            response.raise_for_status()
            
            self.fonts_data = response.json()
            
            # Cache response
            with open(cache_file, 'w', encoding='utf-8') as f:
                json.dump(self.fonts_data, f, indent=2)
            
            print(f"✓ Fetched {len(self.fonts_data.get('items', []))} fonts from API")
            return self.fonts_data
            
        except requests.RequestException as e:
            print(f"✗ API fetch failed: {e}")
            
            # Try to use cache as fallback
            if cache_file.exists():
                print("⚠️  Using cached data as fallback")
                with open(cache_file, 'r', encoding='utf-8') as f:
                    self.fonts_data = json.load(f)
                return self.fonts_data
            
            raise
    
    def get_font_metadata(self, family_name: str) -> Optional[Dict]:
        """
        Get metadata for a specific font family.
        
        Args:
            family_name: Font family name (e.g., 'Roboto', 'Open Sans')
        
        Returns:
            Dict with metadata or None if not found
        """
        if not self.fonts_data:
            self.fetch_all_fonts()
        
        # Search for font (case-insensitive)
        for font in self.fonts_data.get('items', []):
            if font['family'].lower() == family_name.lower():
                return self._extract_metadata(font)
        
        print(f"⚠️  Font '{family_name}' not found in Google Fonts catalog")
        return None
    
    def _extract_metadata(self, font_data: Dict) -> Dict:
        """
        Extract relevant metadata from API response.
        
        Per spec: Separate metadata from geometry.
        
        Args:
            font_data: Raw font data from API
        
        Returns:
            Cleaned metadata dict
        """
        metadata = {
            'family': font_data['family'],
            'category': font_data.get('category', 'unknown'),  # serif, sans-serif, display, etc.
            'variants': font_data.get('variants', []),
            'subsets': font_data.get('subsets', []),
            'version': font_data.get('version', 'unknown'),
            'last_modified': font_data.get('lastModified', 'unknown'),
            'kind': font_data.get('kind', ''),
        }
        
        # Add source attribution
        metadata['source'] = 'Google Fonts API'
        metadata['is_google_font'] = True
        
        return metadata
    
    def enrich_font_profile(self, font_profile: Dict, family_name: str) -> Dict:
        """
        Enrich existing font profile with metadata.
        
        CRITICAL: Adds metadata as separate layer, doesn't modify geometry.
        
        Args:
            font_profile: Existing profile from FontParser
            family_name: Font family name for metadata lookup
        
        Returns:
            Enriched profile with metadata layer
        """
        metadata = self.get_font_metadata(family_name)
        
        if metadata:
            # Add metadata as separate top-level key
            font_profile['metadata_layer'] = metadata
            print(f"✓ Enriched '{family_name}' with Google Fonts metadata")
        else:
            font_profile['metadata_layer'] = {
                'source': 'unknown',
                'is_google_font': False,
                'note': 'Font not found in Google Fonts catalog'
            }
            print(f"⚠️  No metadata found for '{family_name}'")
        
        return font_profile
    
    def batch_enrich_profiles(self, profiles_dir: str = 'fonts/samples') -> List[Dict]:
        """
        Enrich all font profiles in a directory.
        
        Args:
            profiles_dir: Directory containing font files
        
        Returns:
            List of enriched profiles
        """
        profiles_path = Path(profiles_dir)
        font_files = list(profiles_path.glob('*.ttf'))
        
        if not font_files:
            print(f"No TTF fonts found in {profiles_dir}")
            return []
        
        # Fetch catalog once
        self.fetch_all_fonts()
        
        enriched = []
        
        print(f"\n{'='*70}")
        print(f"Enriching {len(font_files)} font profiles with metadata")
        print(f"{'='*70}\n")
        
        # Import FontParser
        import sys
        sys.path.insert(0, 'src')
        from parsers.font_parser import FontParser
        
        for font_file in sorted(font_files):
            try:
                # Parse font
                parser = FontParser(str(font_file))
                profile = parser.extract_full_profile()
                
                # Enrich with metadata
                enriched_profile = self.enrich_font_profile(profile, parser.font_name)
                
                # Save enriched profile
                output_path = font_file.with_suffix('.enriched.json')
                with open(output_path, 'w', encoding='utf-8') as f:
                    json.dump(enriched_profile, f, indent=2, ensure_ascii=False)
                
                print(f"   Saved: {output_path.name}\n")
                
                enriched.append(enriched_profile)
                
                time.sleep(0.5)  # Rate limiting
                
            except Exception as e:
                print(f"✗ Error processing {font_file.name}: {e}\n")
        
        print(f"{'='*70}")
        print(f"✓ Enriched {len(enriched)} font profiles")
        print(f"{'='*70}\n")
        
        return enriched
    
    def get_catalog_statistics(self) -> Dict:
        """
        Get statistics about Google Fonts catalog.
        
        Returns:
            Dict with statistics
        """
        if not self.fonts_data:
            self.fetch_all_fonts()
        
        items = self.fonts_data.get('items', [])
        
        # Category breakdown
        categories = {}
        for font in items:
            cat = font.get('category', 'unknown')
            categories[cat] = categories.get(cat, 0) + 1
        
        # Subset breakdown
        subsets = {}
        for font in items:
            for subset in font.get('subsets', []):
                subsets[subset] = subsets.get(subset, 0) + 1
        
        return {
            'total_fonts': len(items),
            'categories': categories,
            'subsets': subsets,
            'top_10_fonts': [font['family'] for font in items[:10]]
        }


def main():
    """CLI for metadata crawler."""
    import argparse
    
    parser = argparse.ArgumentParser(description='Google Fonts metadata crawler')
    parser.add_argument('--api-key', help='Google Fonts API key (optional)')
    parser.add_argument('--stats', action='store_true', help='Show catalog statistics')
    parser.add_argument('--enrich', metavar='DIR', help='Enrich fonts in directory')
    
    args = parser.parse_args()
    
    crawler = GoogleFontsMetadata(api_key=args.api_key)
    
    if args.stats:
        # Show statistics
        print("\n" + "="*70)
        print("Google Fonts Catalog Statistics")
        print("="*70 + "\n")
        
        stats = crawler.get_catalog_statistics()
        
        print(f"Total fonts: {stats['total_fonts']}\n")
        
        print("Categories:")
        for cat, count in sorted(stats['categories'].items(), key=lambda x: -x[1]):
            print(f"  {cat:20} {count:4}")
        
        print(f"\nTop 10 fonts by popularity:")
        for i, font in enumerate(stats['top_10_fonts'], 1):
            print(f"  {i:2}. {font}")
    
    elif args.enrich:
        # Enrich fonts in directory
        crawler.batch_enrich_profiles(args.enrich)
    
    else:
        # Default: enrich sample fonts
        print("Enriching sample fonts in fonts/samples/")
        crawler.batch_enrich_profiles('fonts/samples')


if __name__ == '__main__':
    main()
