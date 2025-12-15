"""
Alternative Metadata Sources
==============================

Since Google Fonts API requires an API key, we use alternative sources:
1. Google Fonts metadata JSON (publicly available)
2. Web scraping from fonts.google.com
3. FontForge/FontTools embedded metadata
"""

import requests
import json
from typing import Dict, List, Optional
from pathlib import Path
from bs4 import BeautifulSoup
import time


class AlternativeMetadataFetcher:
    """Fetch font metadata from alternative sources."""
    
    # Google Fonts metadata JSON (public, no API key needed)
    METADATA_URL = "https://fonts.google.com/metadata/fonts"
    
    def __init__(self, cache_dir: str = 'data/metadata'):
        """Initialize fetcher."""
        self.cache_dir = Path(cache_dir)
        self.cache_dir.mkdir(parents=True, exist_ok=True)
        self.fonts_data = None
    
    def fetch_metadata_json(self, force_refresh: bool = False) -> Dict:
        """
        Fetch Google Fonts metadata JSON.
        
        This is a publicly available JSON file used by fonts.google.com.
        
        Args:
            force_refresh: Force download instead of using cache
        
        Returns:
            Dict with font metadata
        """
        cache_file = self.cache_dir / 'google_fonts_metadata.json'
        
        # Check cache
        if not force_refresh and cache_file.exists():
            print("✓ Using cached metadata")
            with open(cache_file, 'r', encoding='utf-8') as f:
                self.fonts_data = json.load(f)
            return self.fonts_data
        
        # Fetch metadata JSON
        print("Fetching Google Fonts metadata...")
        
        try:
            headers = {
                'User-Agent': 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36'
            }
            
            response = requests.get(self.METADATA_URL, headers=headers, timeout=30)
            response.raise_for_status()
            
            # Response starts with ")]}'" (XSSI protection), strip it
            text = response.text
            if text.startswith(")]}"):
                text = text[4:]
            
            self.fonts_data = json.loads(text)
            
            # Cache
            with open(cache_file, 'w', encoding='utf-8') as f:
                json.dump(self.fonts_data, f, indent=2)
            
            font_count = len(self.fonts_data.get('familyMetadataList', []))
            print(f"✓ Fetched metadata for {font_count} font families")
            
            return self.fonts_data
            
        except Exception as e:
            print(f"✗ Fetch failed: {e}")
            
            # Fallback to cache
            if cache_file.exists():
                print("Using cached data as fallback")
                with open(cache_file, 'r', encoding='utf-8') as f:
                    self.fonts_data = json.load(f)
                return self.fonts_data
            
            raise
    
    def get_font_metadata(self, family_name: str) -> Optional[Dict]:
        """
        Get metadata for a font family.
        
        Args:
            family_name: Font family name
        
        Returns:
            Metadata dict or None
        """
        if not self.fonts_data:
            self.fetch_metadata_json()
        
        # Search in familyMetadataList
        for font in self.fonts_data.get('familyMetadataList', []):
            if font.get('family', '').lower() == family_name.lower():
                return self._extract_metadata(font)
        
        print(f"⚠️  '{family_name}' not found in metadata")
        return None
    
    def _extract_metadata(self, font_data: Dict) -> Dict:
        """
        Extract relevant metadata.
        
        Google Fonts metadata structure:
        {
          "family": "Roboto",
          "category": "sans-serif",
          "subsets": ["latin", "cyrillic", ...],
          "variants": ["100", "300", "regular", ...],
          "designers": ["Christian Robertson"],
          "dateAdded": "2011-04-13",
          ...
        }
        """
        metadata = {
            'family': font_data.get('family', 'Unknown'),
            'category': font_data.get('category', 'unknown'),
            'designers': font_data.get('designers', []),
            'subsets': font_data.get('subsets', []),
            'variants': font_data.get('variants', []),
            'date_added': font_data.get('dateAdded', 'unknown'),
            'version': font_data.get('version', 'unknown'),
            'last_modified': font_data.get('lastModified', 'unknown'),
        }
        
        # Popularity ranking (if available)
        if 'popularity' in font_data:
            metadata['popularity_rank'] = font_data['popularity']
        
        # Stroke type (if available)
        if 'stroke' in font_data:
            metadata['stroke'] = font_data['stroke']
        
        # Classification (if available)
        if 'classifications' in font_data:
            metadata['classifications'] = font_data['classifications']
        
        metadata['source'] = 'Google Fonts Metadata JSON'
        metadata['is_google_font'] = True
        
        return metadata
    
    def extract_font_internal_metadata(self, font_path: str) -> Dict:
        """
        Extract metadata from font file itself (name table).
        
        Args:
            font_path: Path to font file
        
        Returns:
            Internal metadata
        """
        from fontTools.ttLib import TTFont
        
        try:
            font = TTFont(font_path)
            name_table = font['name']
            
            metadata = {}
            
            # Name IDs:
            # 0: Copyright
            # 1: Font Family
            # 2: Font Subfamily (Regular, Bold, etc.)
            # 3: Unique Font Identifier
            # 4: Full Font Name
            # 5: Version
            # 8: Manufacturer
            # 9: Designer
            # 11: Vendor URL
            # 13: License Description
            # 14: License URL
            
            name_ids = {
                0: 'copyright',
                1: 'family',
                2: 'subfamily',
                4: 'full_name',
                5: 'version',
                8: 'manufacturer',
                9: 'designer',
                11: 'vendor_url',
                13: 'license',
                14: 'license_url',
            }
            
            for name_id, key in name_ids.items():
                for record in name_table.names:
                    if record.nameID == name_id:
                        try:
                            metadata[key] = record.toUnicode()
                            break
                        except:
                            pass
            
            metadata['source'] = 'Font name table (embedded)'
            
            return metadata
            
        except Exception as e:
            print(f"⚠️  Could not extract internal metadata: {e}")
            return {'source': 'font_file', 'error': str(e)}
    
    def enrich_font_profile(self, font_profile: Dict, family_name: str, font_path: Optional[str] = None) -> Dict:
        """
        Enrich font profile with metadata from multiple sources.
        
        Args:
            font_profile: Existing profile from FontParser
            family_name: Font family for lookup
            font_path: Optional path to font file for internal metadata
        
        Returns:
            Enriched profile
        """
        metadata_layers = {}
        
        # 1. Google Fonts metadata
        gf_metadata = self.get_font_metadata(family_name)
        if gf_metadata:
            metadata_layers['google_fonts'] = gf_metadata
            print(f"✓ Added Google Fonts metadata for '{family_name}'")
        
        # 2. Internal font metadata
        if font_path:
            internal_metadata = self.extract_font_internal_metadata(font_path)
            if internal_metadata:
                metadata_layers['font_internal'] = internal_metadata
                print(f"✓ Added internal metadata from font file")
        
        # Add metadata layers to profile
        if metadata_layers:
            font_profile['metadata_layer'] = metadata_layers
        else:
            font_profile['metadata_layer'] = {
                'note': 'No external metadata found',
                'source': 'none'
            }
        
        return font_profile
    
    def batch_enrich_profiles(self, fonts_dir: str = 'fonts/samples') -> List[Dict]:
        """Enrich all fonts in directory."""
        fonts_path = Path(fonts_dir)
        font_files = list(fonts_path.glob('*.ttf'))
        
        # Fetch metadata once
        self.fetch_metadata_json()
        
        enriched = []
        
        print(f"\n{'='*70}")
        print(f"Enriching {len(font_files)} fonts with metadata")
        print(f"{'='*70}\n")
        
        import sys
        sys.path.insert(0, 'src')
        from parsers.font_parser import FontParser
        
        for font_file in sorted(font_files):
            try:
                parser = FontParser(str(font_file))
                profile = parser.extract_full_profile()
                
                # Enrich with both external and internal metadata
                enriched_profile = self.enrich_font_profile(
                    profile,
                    parser.font_name.split()[0],  # Use first word as family name
                    str(font_file)
                )
                
                # Save
                output_path = font_file.with_suffix('.enriched.json')
                with open(output_path, 'w', encoding='utf-8') as f:
                    json.dump(enriched_profile, f, indent=2, ensure_ascii=False)
                
                print(f"   Saved: {output_path.name}\n")
                
                enriched.append(enriched_profile)
                
            except Exception as e:
                print(f"✗ Error: {font_file.name}: {e}\n")
        
        print(f"{'='*70}")
        print(f"✓ Enriched {len(enriched)} profiles")
        print(f"{'='*70}\n")
        
        return enriched


def main():
    """CLI for metadata enrichment."""
    fetcher = AlternativeMetadataFetcher()
    
    # Enrich sample fonts
    fetcher.batch_enrich_profiles('fonts/samples')
    
    # Show statistics
    if fetcher.fonts_data:
        families = fetcher.fonts_data.get('familyMetadataList', [])
        
        print("\n" + "="*70)
        print("Google Fonts Metadata Statistics")
        print("="*70 + "\n")
        print(f"Total font families: {len(families)}\n")
        
        # Category breakdown
        categories = {}
        for font in families:
            cat = font.get('category', 'unknown')
            categories[cat] = categories.get(cat, 0) + 1
        
        print("Categories:")
        for cat, count in sorted(categories.items(), key=lambda x: -x[1]):
            print(f"  {cat:20} {count:4}")


if __name__ == '__main__':
    main()
