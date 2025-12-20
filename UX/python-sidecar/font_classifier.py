"""
Font Classifier - Enhanced Taxonomy System
===========================================

Automatically classify fonts with rich taxonomy:
- Primary class (serif, sans, script, monospace, symbol)
- Sub-classification (didone, geometric, humanist-sans, etc.)
- Secondary tags (anatomy, mood, proportion)
- Role suggestions (body, display, accent, ui)

New System:
- Few classes, many tags
- Class = form (anatomy/shape)
- Tags = character (traits/mood)
- Roles = context (usage)
"""

from typing import Dict, List, Optional


class FontClassifier:
    """Classify fonts with enhanced taxonomy system."""
    
    # Classification thresholds
    MONOSPACE_WIDTH_VAR_MAX = 0.05
    SERIF_CONTRAST_MIN = 2.0
    DIDONE_CONTRAST_MIN = 3.5  # High contrast serif
    SLAB_CONTRAST_MAX = 1.8    # Low contrast serif
    
    @staticmethod
    def _get_metric(metrics: dict, key: str, default: float = 0.0) -> float:
        """Safely extract metric value - handles both float and dict formats."""
        value = metrics.get(key, default)
        if value is None:
            return default
        if isinstance(value, (int, float)):
            return float(value)
        if isinstance(value, dict):
            # Try common dict keys
            for subkey in ['mean_contrast', 'mean_stress', 'mean_aperture', 
                          'mean_curvature', 'mean_sharpness', 'mean']:
                if subkey in value:
                    return float(value[subkey])
            return default
        return default
    
    @staticmethod
    def classify(profile: Dict) -> str:
        """
        Legacy classifier for backward compatibility.
        Returns simple category string.
        """
        result = FontClassifier.classify_enhanced(profile)
        return result['primary_class']
    
    @staticmethod
    def classify_enhanced(profile: Dict) -> Dict:
        """
        Enhanced classifier with full taxonomy.
        
        Returns:
            {
                'primary_class': 'sans',
                'secondary_tags': ['neo-grotesque', 'low-contrast', 'clean'],
                'sans_subtype': 'neo-grotesque',
                'suitable_roles': ['body', 'ui']
            }
        """
        result = {
            'primary_class': None,
            'secondary_tags': [],
            'serif_subtype': None,
            'sans_subtype': None,
            'script_subtype': None,
            'suitable_roles': []
        }
        
        # Step 1: Determine primary class
        primary_class = FontClassifier._determine_primary_class(profile)
        result['primary_class'] = primary_class
        
        # Step 2: Determine subtype
        if primary_class == 'serif':
            subtype = FontClassifier._determine_serif_subtype(profile)
            result['serif_subtype'] = subtype
            if subtype:
                result['secondary_tags'].append(subtype)
        elif primary_class == 'sans':
            subtype = FontClassifier._determine_sans_subtype(profile)
            result['sans_subtype'] = subtype
            if subtype:
                result['secondary_tags'].append(subtype)
        elif primary_class == 'script':
            subtype = FontClassifier._determine_script_subtype(profile)
            result['script_subtype'] = subtype
            if subtype:
                result['secondary_tags'].append(subtype)
        
        # Step 3: Generate anatomy tags
        anatomy_tags = FontClassifier._generate_anatomy_tags(profile)
        result['secondary_tags'].extend(anatomy_tags)
        
        # Step 4: Generate mood tags
        mood_tags = FontClassifier._generate_mood_tags(profile, primary_class)
        result['secondary_tags'].extend(mood_tags)
        
        # Step 5: Determine suitable roles
        result['suitable_roles'] = FontClassifier._determine_roles(result, profile)
        
        return result
    
    @staticmethod
    def _determine_primary_class(profile: Dict) -> str:
        """
        Determine primary classification using ONLY metrics.
        
        Key insight from research:
        - Curvature > 0.70 is the BEST script indicator
        - High-contrast sans (display fonts) need curvature check to avoid serif misclassification
        - PANOSE is used only for symbol fonts (bFamilyType=5)
        """
        basic = profile.get('basic_metrics', {})
        anatomy = profile.get('anatomy_metrics', {})
        panose = profile.get('panose')
        
        # Extract key metrics
        width_variance = basic.get('width_variance', 0.15)
        stroke_contrast = FontClassifier._get_metric(anatomy, 'stroke_contrast', 1.0)
        curvature = FontClassifier._get_metric(anatomy, 'curvature_ratio', 0.4)
        aperture = FontClassifier._get_metric(anatomy, 'aperture', 0.5)
        
        # 1. Symbol fonts via PANOSE only
        if panose and len(panose) >= 1:
            if panose[0] == 5:  # Symbol
                return 'symbol'
        
        # 2. Monospace (very low width variance is highly distinctive)
        if width_variance is not None and width_variance < FontClassifier.MONOSPACE_WIDTH_VAR_MAX:
            return 'monospace'
        
        # 3. Script detection (MULTI-FACTOR - before serif/sans)
        # Using tiered approach for more reliable detection:
        #   Tier 1: Very high curvature (>0.80) = definitely script
        #   Tier 2: High curvature + high width variance = likely script
        #   Tier 3: Moderate-high curvature (>0.60) with supporting factors
        
        # Tier 1: Very high curvature is a definite script indicator
        if curvature > 0.80:
            return 'script'
        
        # Tier 2: Combination of high curvature + high width variance
        # Scripts have both flowing curves AND variable letterforms
        if curvature > 0.55 and width_variance > 0.20:
            return 'script'
        
        # Tier 3: Moderate-high curvature for brush-style fonts
        # Fonts like LeviBrush (curv=0.63) have distinctive flowing forms
        if curvature > 0.60:
            return 'script'
        
        # 4. Serif vs Sans classification
        # Need to distinguish true serifs from high-contrast display sans
        # True serif: has serifs (high contrast) + moderate curvature (not too angular)
        # Display sans: may have high contrast but is angular (low curvature)
        
        # Very high contrast + moderate curvature = definitely serif
        if stroke_contrast > 3.0 and curvature > 0.35:
            return 'serif'
        
        # High contrast + not too angular = likely serif
        if stroke_contrast >= 2.5 and curvature > 0.40:
            return 'serif'
        
        # Moderate contrast + rounded forms = serif
        if stroke_contrast >= 2.0 and curvature > 0.50:
            return 'serif'
        
        # Everything else is sans-serif
        # (low contrast OR angular forms regardless of contrast)
        return 'sans'
    
    @staticmethod
    def _determine_serif_subtype(profile: Dict) -> Optional[str]:
        """
        Determine serif sub-classification using ONLY metrics.
        No font name heuristics - purely typographic analysis.
        
        Serif subtypes based on stroke contrast and stress angle:
        - Slab: Low contrast (<1.8), no visible thick/thin variation
        - Didone: Very high contrast (>3.5), vertical stress
        - Transitional: Medium-high contrast (2.5-3.5), near-vertical stress
        - Old-style: Moderate contrast, diagonal stress (>10° from vertical)
        - Humanist: Default for serifs that don't fit other categories
        """
        anatomy = profile.get('anatomy_metrics', {})
        panose = profile.get('panose')
        
        stroke_contrast = FontClassifier._get_metric(anatomy, 'stroke_contrast', 2.0)
        stress_angle = FontClassifier._get_metric(anatomy, 'stress_angle', 90)
        aperture = FontClassifier._get_metric(anatomy, 'aperture', 0.5)
        
        # 1. Check PANOSE for slab serif (reliable when set)
        if panose and len(panose) >= 2:
            bSerifStyle = panose[1]
            if bSerifStyle == 6:  # Slab
                return 'slab'
        
        # 2. Slab: Very low contrast serif (check FIRST)
        # Slab serifs have minimal thick/thin variation
        if stroke_contrast < FontClassifier.SLAB_CONTRAST_MAX:
            return 'slab'
        
        # 3. Didone: Very high contrast with vertical stress
        # Didone/Modern fonts have extreme thick/thin contrast
        if stroke_contrast > FontClassifier.DIDONE_CONTRAST_MIN:
            if abs(stress_angle - 90) <= 15:  # Near vertical stress
                return 'didone'
        
        # 4. Old-Style: Has diagonal stress (tilted axis in curved strokes)
        # Old-style serifs have a calligraphic, angled stress
        if abs(stress_angle - 90) > 15:  # Diagonal stress
            return 'old-style'
        
        # 5. Transitional: Medium-high contrast, vertical stress
        # Between old-style and modern characteristics
        if stroke_contrast > 2.5:
            return 'transitional'
        
        # Default: Humanist serif (moderate everything)
        return 'humanist-serif'
    
    @staticmethod
    def _determine_sans_subtype(profile: Dict) -> Optional[str]:
        """
        Determine sans-serif sub-classification using metrics.
        
        Sans subtypes:
        - Geometric: Based on geometric shapes (circles, squares), uniform strokes
        - Humanist-sans: Calligraphic influence, more contrast variation
        - Grotesque: Early sans-serifs, some contrast, closed apertures
        - Neo-grotesque: Highly uniform, minimal contrast, open forms
        """
        basic = profile.get('basic_metrics', {})
        anatomy = profile.get('anatomy_metrics', {})
        
        x_height_ratio = basic.get('optical_x_ratio', 0.5)
        aperture = FontClassifier._get_metric(anatomy, 'aperture', 0.5)
        stroke_contrast = FontClassifier._get_metric(anatomy, 'stroke_contrast', 1.0)
        curvature = FontClassifier._get_metric(anatomy, 'curvature_ratio', 0.4)
        
        # Geometric: Based on perfect geometric forms
        # Tall x-height, very open aperture, very low contrast, round curves
        if x_height_ratio > 0.58 and aperture > 0.6 and stroke_contrast < 1.15 and curvature > 0.45:
            return 'geometric'
        
        # Humanist Sans: Has calligraphic/pen influence
        # Noticeable stroke contrast variation (> 1.2)
        if stroke_contrast > 1.2:
            return 'humanist-sans'
        
        # Neo-Grotesque: Highly refined, uniform, neutral
        # Very low contrast, high x-height, open apertures
        if stroke_contrast < 1.15 and aperture > 0.55:
            return 'neo-grotesque'
        
        # Grotesque (Grotesk): Early sans-serifs, less refined
        # Moderate contrast, closed apertures, some quirks
        # This is the default for sans that doesn't fit above categories
        return 'grotesque'
    
    @staticmethod
    def _determine_script_subtype(profile: Dict) -> Optional[str]:
        """Determine script sub-classification."""
        anatomy = profile.get('anatomy_metrics', {})
        
        curvature = FontClassifier._get_metric(anatomy, 'curvature_ratio', 0.5)
        terminal = FontClassifier._get_metric(anatomy, 'terminal_sharpness', 0.5)
        
        # Brush: High curvature, soft terminals
        if curvature > 0.7 and terminal < 0.4:
            return 'brush'
        
        # Formal: Low curvature, sharp terminals
        if curvature < 0.5 and terminal > 0.6:
            return 'formal'
        
        # Default: Casual handwritten
        return 'casual'
    
    @staticmethod
    def _generate_anatomy_tags(profile: Dict) -> List[str]:
        """Generate anatomy-based tags."""
        tags = []
        anatomy = profile.get('anatomy_metrics', {})
        basic = profile.get('basic_metrics', {})
        
        stroke_contrast = FontClassifier._get_metric(anatomy, 'stroke_contrast', 1.0)
        aperture = FontClassifier._get_metric(anatomy, 'aperture', 0.5)
        x_height_ratio = basic.get('optical_x_ratio', 0.5)
        width_variance = basic.get('width_variance', 0)
        curvature = FontClassifier._get_metric(anatomy, 'curvature_ratio', 0.4)
        terminal = FontClassifier._get_metric(anatomy, 'terminal_sharpness', 0.5)
        
        # Contrast tags
        if stroke_contrast < 1.3:
            tags.append('low-contrast')
        elif stroke_contrast < 2.5:
            tags.append('medium-contrast')
        elif stroke_contrast < 3.5:
            tags.append('high-contrast')
        else:
            tags.append('extreme-contrast')
        
        # Aperture tags
        if aperture > 0.6:
            tags.append('open-aperture')
        elif aperture < 0.4:
            tags.append('closed-aperture')
        
        # X-height tags
        if x_height_ratio > 0.58:
            tags.append('tall-x-height')
        elif x_height_ratio < 0.45:
            tags.append('short-x-height')
        
        # Width tags
        if width_variance > 0.3:
            tags.append('varied-width')
        elif width_variance < 0.05:
            tags.append('fixed-width')
        
        # Curvature/form tags (square vs rounded)
        if curvature < 0.30:
            tags.append('square')      # Angular, boxy letterforms
            tags.append('angular')
        elif curvature > 0.55:
            tags.append('rounded')     # Soft, rounded corners/terminals
            tags.append('soft')
        
        # Terminal tags
        if terminal > 0.7:
            tags.append('sharp-terminals')
        elif terminal < 0.3:
            tags.append('soft-terminals')
        
        return tags
    
    @staticmethod
    def _generate_mood_tags(profile: Dict, primary_class: str) -> List[str]:
        """Generate mood/vibe tags."""
        tags = []
        anatomy = profile.get('anatomy_metrics', {})
        
        stroke_contrast = FontClassifier._get_metric(anatomy, 'stroke_contrast', 1.0)
        aperture = FontClassifier._get_metric(anatomy, 'aperture', 0.5)
        
        # Clean/Modern
        if primary_class == 'sans' and stroke_contrast < 1.2 and aperture > 0.5:
            tags.extend(['clean', 'modern'])
        
        # Classic/Editorial
        if primary_class == 'serif' and 2.0 < stroke_contrast < 3.5:
            tags.extend(['classic', 'editorial'])
        
        # Luxury/Elegant
        if stroke_contrast > 3.5:
            tags.extend(['luxury', 'elegant'])
        
        # Tech/Neutral
        if primary_class == 'sans' and stroke_contrast < 1.15:
            tags.append('neutral')
        
        if primary_class == 'monospace':
            tags.append('tech')
        
        # Warm
        if primary_class in ['serif', 'script'] and stroke_contrast < 2.5:
            tags.append('warm')
        
        return tags
    
    @staticmethod
    def _determine_roles(result: Dict, profile: Dict) -> List[str]:
        """Determine suitable usage roles."""
        roles = []
        primary_class = result['primary_class']
        tags = result['secondary_tags']
        
        anatomy = profile.get('anatomy_metrics', {})
        basic = profile.get('basic_metrics', {})
        
        stroke_contrast = FontClassifier._get_metric(anatomy, 'stroke_contrast', 1.0)
        aperture = FontClassifier._get_metric(anatomy, 'aperture', 0.5)
        x_height_ratio = basic.get('optical_x_ratio', 0.5)
        
        # Body role candidates
        if primary_class == 'sans' and 'humanist-sans' in tags:
            roles.append('body')
        elif primary_class == 'sans' and aperture > 0.5 and x_height_ratio > 0.48:
            roles.append('body')
        elif primary_class == 'serif' and stroke_contrast < 2.5 and 'open-aperture' in tags:
            roles.append('body')
        
        # Display role candidates
        if stroke_contrast > 2.5 or 'tall-x-height' in tags:
            roles.append('display')
        if primary_class == 'serif' and 'didone' in tags:
            roles.append('display')
        
        # Accent role candidates
        if primary_class == 'serif':
            roles.append('accent')
        elif primary_class == 'sans' and 'humanist-sans' in tags and stroke_contrast > 1.3:
            roles.append('accent')
        
        # UI role candidates
        if primary_class == 'sans' and aperture > 0.6 and x_height_ratio > 0.52:
            roles.append('ui')
        elif primary_class == 'monospace':
            roles.append('ui')
        
        return roles
    
    @staticmethod
    def classify_with_confidence(profile: Dict) -> Dict:
        """
        Legacy method with confidence scoring.
        Enhanced to use new taxonomy.
        """
        enhanced = FontClassifier.classify_enhanced(profile)
        
        # Build confidence based on primary class
        confidence = 0.75  # Base confidence
        primary = enhanced['primary_class']
        tags = enhanced['secondary_tags']
        
        reasoning = []
        
        if primary == 'monospace':
            confidence = 0.95
            reasoning.append('Fixed-width detection')
        elif primary == 'serif':
            if enhanced['serif_subtype']:
                reasoning.append(f"Serif: {enhanced['serif_subtype']}")
                confidence = 0.85
            else:
                reasoning.append('Serif classification')
                confidence = 0.75
        elif primary == 'sans':
            if enhanced['sans_subtype']:
                reasoning.append(f"Sans: {enhanced['sans_subtype']}")
                confidence = 0.80
            else:
                reasoning.append('Sans classification')
                confidence = 0.70
        
        # Add tag hints
        if 'low-contrast' in tags:
            reasoning.append('low contrast')
        if 'high-contrast' in tags or 'extreme-contrast' in tags:
            reasoning.append('high contrast')
        
        return {
            'category': primary,
            'confidence': confidence,
            'reasoning': ' | '.join(reasoning),
            'enhanced': enhanced
        }


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
    
    result = FontClassifier.classify_enhanced(profile)
    
    print(f"\n{'='*60}")
    print(f"Font Enhanced Classification: {profile_path.stem}")
    print(f"{'='*60}\n")
    print(f"Primary Class: {result['primary_class'].upper()}")
    
    if result.get('serif_subtype'):
        print(f"Serif Subtype: {result['serif_subtype']}")
    if result.get('sans_subtype'):
        print(f"Sans Subtype: {result['sans_subtype']}")
    if result.get('script_subtype'):
        print(f"Script Subtype: {result['script_subtype']}")
    
    print(f"\nSecondary Tags: {', '.join(result['secondary_tags'])}")
    print(f"Suitable Roles: {', '.join(result['suitable_roles'])}")
    
    print(f"\n{'='*60}\n")
