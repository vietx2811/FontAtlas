"""
Font Classifier
================

Automatically classify fonts into categories based on extracted metrics.

Categories:
- serif: Traditional fonts with serifs and high stroke contrast
- sans-serif: Modern fonts with minimal contrast
- display: Decorative fonts for headings
- monospace: Fixed-width fonts
- script: Handwriting/calligraphic fonts
- decorative: Unusual/artistic fonts

Algorithm uses decision tree based on:
1. Width variance → Monospace detection
2. Stroke contrast → Serif vs Sans
3. Aperture + X-height → Display vs Text
4. Extreme metrics → Decorative
"""

from typing import Dict, Optional


class FontClassifier:
    """Classify fonts based on extracted metrics."""
    
    # Classification thresholds
    MONOSPACE_WIDTH_VAR_MAX = 0.05  # Very uniform widths
    SERIF_CONTRAST_MIN = 2.0        # Clear thick/thin variation
    DISPLAY_APERTURE_MAX = 0.35     # Narrow apertures
    DISPLAY_XHEIGHT_MIN = 0.60      # Tall x-height (uncommon in text)
    EXTREME_CONTRAST_MIN = 4.0      # Very high contrast = decorative
    
    @staticmethod
    def classify(profile: Dict) -> str:
        """
        Classify font based on complete profile.
        
        Args:
            profile: Complete font profile with metrics
            
        Returns:
            Category string: 'serif', 'sans-serif', 'display', 'monospace', 'script', 'decorative'
        """
        basic = profile.get('basic_metrics', {})
        anatomy = profile.get('anatomy_metrics', {})
        
        # Extract key metrics
        width_variance = basic.get('width_variance', 0)
        stroke_contrast = anatomy.get('stroke_contrast', {}).get('mean_contrast', 1.0)
        aperture = anatomy.get('aperture_openness', {}).get('mean_aperture', 0.5)
        x_height_ratio = basic.get('optical_x_ratio', 0.5)
        x_height_source = basic.get('x_height_source', 'unknown')
        
        # Decision tree classification
        
        # 0. Check PANOSE data if available (Golden Source)
        panose = profile.get('panose')
        if panose and len(panose) >= 2:
            bFamilyType = panose[0]
            bSerifStyle = panose[1]
            
            # Family Type: 2=Latin Text, 3=Latin Script, 4=Latin Decorative, 5=Latin Pictorial
            if bFamilyType == 3: return 'script'
            if bFamilyType == 4: return 'display' # or decorative
            
            # Serif Style (for Latin Text)
            # 2-10 = Various Serifs
            # 11-15 = Various Sans (Normal, Additive, Obtuse, etc)
            if bFamilyType == 2:
                if 2 <= bSerifStyle <= 10: return 'serif'
                if 11 <= bSerifStyle <= 15: return 'sans-serif'
        
        # 1. Check for Monospace (most distinctive)
        if width_variance is not None and width_variance < FontClassifier.MONOSPACE_WIDTH_VAR_MAX:
            return 'monospace'
        
        # 2. Check for extreme/decorative characteristics
        if stroke_contrast > FontClassifier.EXTREME_CONTRAST_MIN:
            # Very high contrast = Didone or decorative
            return 'display'  # Often used for display
        
        # 3. Check for Display fonts (narrow apertures, unusual proportions)
        if aperture is not None and aperture < FontClassifier.DISPLAY_APERTURE_MAX:
            return 'display'
        
        # Tall x-height can indicate display (geometric sans)
        if x_height_ratio > FontClassifier.DISPLAY_XHEIGHT_MIN:
            # But only if sans-serif (low contrast)
            if stroke_contrast < 1.5:
                return 'display'
        
        # 4. Serif vs Sans-Serif (main classification)
        if stroke_contrast >= FontClassifier.SERIF_CONTRAST_MIN:
            return 'serif'
        else:
            return 'sans-serif'
    
    @staticmethod
    def classify_with_confidence(profile: Dict) -> Dict[str, any]:
        """
        Classify with confidence score.
        
        Returns:
            {
                'category': 'serif',
                'confidence': 0.85,
                'alternative': 'display',
                'reasoning': 'High stroke contrast (2.3) suggests serif'
            }
        """
        basic = profile.get('basic_metrics', {})
        anatomy = profile.get('anatomy_metrics', {})
        
        width_variance = basic.get('width_variance', 0)
        stroke_contrast = anatomy.get('stroke_contrast', {}).get('mean_contrast', 1.0)
        aperture = anatomy.get('aperture_openness', {}).get('mean_aperture', 0.5)
        x_height_ratio = basic.get('optical_x_ratio', 0.5)
        
        category = FontClassifier.classify(profile)
        confidence = 0.5
        reasoning = []
        alternative = None
        
        # Calculate confidence based on how strongly metrics support the category
        
        if category == 'monospace':
            confidence = 0.95  # Very reliable
            reasoning.append(f'Very uniform width variance ({width_variance:.3f})')
        
        elif category == 'serif':
            if stroke_contrast > 3.0:
                confidence = 0.90
                reasoning.append(f'High stroke contrast ({stroke_contrast:.2f})')
            elif stroke_contrast > 2.5:
                confidence = 0.80
                reasoning.append(f'Moderate-high stroke contrast ({stroke_contrast:.2f})')
            else:
                confidence = 0.65
                reasoning.append(f'Borderline stroke contrast ({stroke_contrast:.2f})')
                alternative = 'sans-serif'
        
        elif category == 'sans-serif':
            if stroke_contrast < 1.3:
                confidence = 0.90
                reasoning.append(f'Low stroke contrast ({stroke_contrast:.2f})')
            elif stroke_contrast < 1.8:
                confidence = 0.75
                reasoning.append(f'Moderate-low stroke contrast ({stroke_contrast:.2f})')
            else:
                confidence = 0.60
                reasoning.append(f'Borderline stroke contrast ({stroke_contrast:.2f})')
                alternative = 'serif'
        
        elif category == 'display':
            if stroke_contrast > FontClassifier.EXTREME_CONTRAST_MIN:
                confidence = 0.85
                reasoning.append(f'Extreme stroke contrast ({stroke_contrast:.2f})')
            elif aperture < FontClassifier.DISPLAY_APERTURE_MAX:
                confidence = 0.75
                reasoning.append(f'Narrow aperture ({aperture:.2f})')
            else:
                confidence = 0.70
                reasoning.append(f'Display characteristics detected')
        
        result = {
            'category': category,
            'confidence': confidence,
            'reasoning': ' | '.join(reasoning)
        }
        
        if alternative:
            result['alternative'] = alternative
        
        return result


# CLI for testing
if __name__ == '__main__':
    import sys
    import json
    from pathlib import Path
    
    if len(sys.argv) < 2:
        print("Usage: python font_classifier.py <profile.json>")
        sys.exit(1)
    
    profile_path = Path(sys.argv[1])
    
    if not profile_path.exists():
        print(f"Error: Profile not found: {profile_path}")
        sys.exit(1)
    
    with open(profile_path) as f:
        profile = json.load(f)
    
    result = FontClassifier.classify_with_confidence(profile)
    
    print(f"\n{'='*60}")
    print(f"Font Classification: {profile_path.stem}")
    print(f"{'='*60}\n")
    print(f"Category: {result['category'].upper()}")
    print(f"Confidence: {result['confidence']*100:.0f}%")
    print(f"Reasoning: {result['reasoning']}")
    
    if 'alternative' in result:
        print(f"Alternative: {result['alternative']}")
    
    print(f"\n{'='*60}\n")
