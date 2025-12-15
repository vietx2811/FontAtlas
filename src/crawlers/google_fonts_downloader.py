"""
Download fonts directly from Google Fonts API
===============================================

Using Google Fonts API v1 to fetch font files.
"""

import requests
import json
from pathlib import Path
from typing import Dict, List
import time


class GoogleFontsAPIDownloader:
    """Download fonts using Google Fonts Developer API."""
    
    API_BASE = "https://www.googleapis.com/webfonts/v1/webfonts"
    
    # Test fonts with verified family names
    TEST_FONTS = [
        {'family': 'Roboto', 'desc': 'geometric sans, low contrast'},
        {'family': 'Playfair Display', 'desc': 'transitional serif, high contrast'},
        {'family': 'Source Code Pro', 'desc': 'monospace'},
        {'family': 'Open Sans', 'desc': 'humanist sans'},
        {'family': 'Libre Baskerville', 'desc': 'transitional serif'},
    ]
    
    def __init__(self, api_key: str = None, output_dir: str = 'fonts/samples'):
        """
        Initialize downloader.
        
        Args:
            api_key: Google Fonts API key (optional for basic download)
            output_dir: Directory to save fonts
        """
        self.api_key = api_key
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)
    
    def download_font_direct(self, family: str) -> Path:
        """
        Download font using direct CSS API trick.
        
        Google Fonts CSS API returns direct TTF URLs.
        
        Args:
            family: Font family name
        
        Returns:
            Path to downloaded file
        """
        # Use Google Fonts CSS API to get font URLs
        family_encoded = family.replace(' ', '+')
        css_url = f"https://fonts.googleapis.com/css2?family={family_encoded}"
        
        try:
            # Get CSS with font URLs
            response = requests.get(css_url, headers={
                'User-Agent': 'Mozilla/5.0'  # Needed to get TTF instead of WOFF2
            })
            response.raise_for_status()
            
            # Parse CSS to extract TTF URL
            css_text = response.text
            
            # Find URL in CSS
            import re
            url_match = re.search(r'url\((https://[^\)]+\.ttf)\)', css_text)
            
            if not url_match:
                print(f"⚠️  Could not find TTF URL for {family}")
                return Path('')
            
            font_url = url_match.group(1)
            
            # Download TTF
            font_response = requests.get(font_url)
            font_response.raise_for_status()
            
            # Save
            filename = f"{family.lower().replace(' ', '_')}.ttf"
            output_path = self.output_dir / filename
            
            with open(output_path, 'wb') as f:
                f.write(font_response.content)
            
            print(f"✓ Downloaded {family} → {filename}")
            return output_path
            
        except Exception as e:
            print(f"✗ Failed to download {family}: {e}")
            return Path('')
    
    def download_test_set(self) -> List[Path]:
        """Download all test fonts."""
        downloaded = []
        
        print(f"\n{'='*60}")
        print("Downloading Google Fonts (via CSS API)...")
        print(f"{'='*60}\n")
        
        for font_info in self.TEST_FONTS:
            path = self.download_font_direct(font_info['family'])
            if path and path.exists():
                downloaded.append(path)
            time.sleep(1)  # Rate limiting
        
        print(f"\n✓ Downloaded {len(downloaded)} / {len(self.TEST_FONTS)} fonts\n")
        return downloaded
    
    def create_manifest(self):
        """Create manifest of downloaded fonts."""
        manifest = {
            'fonts': self.TEST_FONTS,
            'source': 'Google Fonts CSS API',
            'total': len(self.TEST_FONTS),
        }
        
        manifest_path = self.output_dir / 'manifest.json'
        with open(manifest_path, 'w') as f:
            json.dump(manifest, f, indent=2)
        
        print(f"✓ Manifest: {manifest_path}\n")


def main():
    """Download test fonts."""
    downloader = GoogleFontsAPIDownloader()
    
    # Download
    fonts = downloader.download_test_set()
    
    # Create manifest
    downloader.create_manifest()
    
    # Test parser if fonts were downloaded
    if fonts:
        print(f"{'='*60}")
        print("Testing FontParser on first downloaded font...")
        print(f"{'='*60}\n")
        
        # Import parser
        import sys
        sys.path.insert(0, 'src')
        from parsers.font_parser import FontParser
        
        # Test on first font
        test_font = fonts[0]
        parser = FontParser(str(test_font))
        
        metrics = parser.get_basic_metrics()
        coverage = parser.get_unicode_coverage()
        
        print(f"✓ Parser working on {parser.font_name}")
        print(f"  x-height ratio: {metrics.get('x_height_ratio', 'N/A')}")
        print(f"  Vietnamese support: {'✓' if coverage.get('supports_vietnamese') else '✗'}\n")


if __name__ == '__main__':
    main()
