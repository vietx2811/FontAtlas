"""
Complete Font Profile Integrator
==================================

Integrate all layers: geometry + coverage + metadata + perceptual embeddings.

Per spec vector composition:
- Geometry: 55% (not yet fully implemented - using basic metrics as proxy)
- Perceptual: 30%
- Functional: 15%

For Phase 1 (OSS features), we have:
- Basic metrics (proxy for geometry) ✓
- Perceptual embeddings ✓
- Functional stats (coverage) ✓
"""

import json
from pathlib import Path
from typing import Dict, List
import numpy as np
import sys

sys.path.insert(0, 'src')


class FontProfileIntegrator:
    """Integrate all font analysis layers."""
    
    def __init__(self):
        """Initialize integrator."""
        pass
    
    def create_complete_profile(self, font_path: str) -> Dict:
        """
        Create complete font profile with all layers.
        
        Layers:
        1. Basic metrics (x-height, width variance)
        2. Unicode coverage
        3. Metadata (Google Fonts + internal)
        4. Perceptual embeddings (OpenCLIP)
        5. **Custom anatomy metrics** (stroke contrast, aperture) ← Phase 3
        
        Args:
            font_path: Path to font file
        
        Returns:
            Dict with all layers integrated
        """
        from parsers.font_parser import FontParser
        from crawlers.metadata_fetcher import AlternativeMetadataFetcher
        from embeddings.perceptual_embedder import PerceptualEmbedder
        from metrics.stroke_contrast import StrokeContrastAnalyzer
        from metrics.aperture_detector import ApertureDetector
        
        font_path = Path(font_path)
        print(f"\n{'='*70}")
        print(f"Creating complete profile: {font_path.name}")
        print(f"{'='*70}\n")
        
        # 1. Basic metrics + coverage
        print("1/5 Extracting basic metrics...")
        parser = FontParser(str(font_path))
        profile = parser.extract_full_profile()
        
        # 2. Metadata enrichment
        print("2/5 Enriching with metadata...")
        metadata_fetcher = AlternativeMetadataFetcher()
        family_name = parser.font_name.split()[0]  # First word as family
        profile = metadata_fetcher.enrich_font_profile(profile, family_name, str(font_path))
        
        # 3. Perceptual embeddings
        print("3/5 Extracting perceptual embeddings...")
        embedder = PerceptualEmbedder(model_name='ViT-B-32', pretrained='openai')
        perceptual_data = embedder.embed_font(str(font_path))
        
        # Add perceptual layer to profile
        profile['perceptual_layer'] = perceptual_data
        
        # 4. Stroke contrast analysis (Phase 3)
        print("4/6 Analyzing stroke contrast...")
        try:
            contrast_analyzer = StrokeContrastAnalyzer(str(font_path))
            contrast_results = contrast_analyzer.analyze_font_contrast()
            profile['anatomy_metrics'] = {
                'stroke_contrast': contrast_results
            }
        except Exception as e:
            print(f"   ⚠️  Stroke contrast analysis failed: {e}")
            profile['anatomy_metrics'] = {'stroke_contrast': {'error': str(e)}}
        
        # 5. Aperture detection (Phase 3)
        print("5/6 Detecting apertures...")
        try:
            aperture_detector = ApertureDetector(str(font_path))
            aperture_results = aperture_detector.analyze_font_aperture()
            profile['anatomy_metrics']['aperture_openness'] = aperture_results
        except Exception as e:
            print(f"   ⚠️  Aperture detection failed: {e}")
            if 'anatomy_metrics' not in profile:
                profile['anatomy_metrics'] = {}
            profile['anatomy_metrics']['aperture_openness'] = {'error': str(e)}
        
        # 6. Automatic category classification
        print("6/6 Classifying font category...")
        try:
            from metrics.font_classifier import FontClassifier
            classification_result = FontClassifier.classify_with_confidence(profile)
            
            # Add to metadata layer
            if 'metadata_layer' not in profile:
                profile['metadata_layer'] = {}
            
            # Use classified category, but preserve Google Fonts category if exists
            existing_category = profile.get('metadata_layer', {}).get('google_fonts', {}).get('category')
            
            profile['metadata_layer']['classification'] = {
                'auto_category': classification_result['category'],
                'confidence': classification_result['confidence'],
                'reasoning': classification_result['reasoning'],
                'google_fonts_category': existing_category
            }
            
            # Set the primary category (prefer auto-classification)
            if not existing_category or existing_category == 'Unknown':
                profile['metadata_layer']['category'] = classification_result['category']
            
            print(f"   ✓ Classified as: {classification_result['category']} ({classification_result['confidence']*100:.0f}% confidence)")
            
        except Exception as e:
            print(f"   ⚠️  Classification failed: {e}")
        
        print(f"\n✓ Complete profile created with Phase 3 anatomy metrics!\n")
        
        return profile
    
    def batch_create_profiles(self, fonts_dir: str = 'fonts/samples') -> List[Dict]:
        """
        Create complete profiles for all fonts in directory.
        
        Args:
            fonts_dir: Directory containing font files
        
        Returns:
            List of complete profiles
        """
        fonts_path = Path(fonts_dir)
        font_files = list(fonts_path.glob('*.ttf'))
        
        if not font_files:
            print(f"No TTF fonts found in {fonts_dir}")
            return []
        
        print(f"\n{'='*70}")
        print(f"Batch Processing: {len(font_files)} fonts")
        print(f"{'='*70}\n")
        
        profiles = []
        
        for i, font_file in enumerate(sorted(font_files), 1):
            print(f"\n[{i}/{len(font_files)}] {font_file.name}")
            print(f"{'-'*70}")
            
            try:
                profile = self.create_complete_profile(str(font_file))
                
                # Save complete profile
                output_path = font_file.with_suffix('.complete.json')
                with open(output_path, 'w', encoding='utf-8') as f:
                    json.dump(profile, f, indent=2, ensure_ascii=False)
                
                print(f"✓ Saved: {output_path.name}\n")
                
                profiles.append(profile)
                
            except Exception as e:
                print(f"✗ Error: {e}\n")
        
        print(f"\n{'='*70}")
        print(f"✓ Batch complete: {len(profiles)} profiles created")
        print(f"{'='*70}\n")
        
        return profiles


def main():
    """CLI for profile integration."""
    integrator = FontProfileIntegrator()
    
    # Batch process all fonts
    profiles = integrator.batch_create_profiles('fonts/samples')
    
    # Summary
    if profiles:
        print("\n📊 Summary:")
        print(f"   Total profiles: {len(profiles)}")
        print(f"   Layers per profile: 5 (metrics, coverage, metadata, perceptual, anatomy)")
        print(f"   Perceptual embedding dim: 512")
        print(f"   Anatomy metrics: stroke contrast, aperture openness")
        print("\n✓ Phase 1-3 Integration COMPLETE!")


if __name__ == '__main__':
    main()
