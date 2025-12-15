"""
Font Pairing Scorer
====================

Multi-factor scoring algorithm for font compatibility.

Scoring Factors (Per Research):
- X-height harmony: 40% weight
- Width variance complementarity: 30% weight 
- Perceptual similarity: 20% weight
- Category diversity: 10% weight
"""

import numpy as np
from typing import Dict, Tuple


class PairingScorer:
    """Calculate compatibility scores between font pairs."""
    
    # Scoring weights (updated for Phase 3 with anatomy metrics)
    WEIGHT_X_HEIGHT = 0.30        # Reduced from 0.40
    WEIGHT_WIDTH_VAR = 0.20       # Reduced from 0.30
    WEIGHT_STROKE_CONTRAST = 0.15 # NEW - Phase 3
    WEIGHT_APERTURE = 0.05        # NEW - Phase 3
    WEIGHT_PERCEPTUAL = 0.20      # Same
    WEIGHT_CATEGORY = 0.10        # Same
    
    # Thresholds
    X_HEIGHT_IDEAL_MAX_DIFF = 0.15  # <15% difference = good harmony
    WIDTH_VAR_IDEAL_MIN_DIFF = 0.05  # Minimum for complementarity
    WIDTH_VAR_IDEAL_MAX_DIFF = 0.15  # Maximum before too different
    PERCEPTUAL_SIM_MIN = 0.90  # Sweet spot range
    PERCEPTUAL_SIM_MAX = 0.98
    
    # Phase 3: Anatomy thresholds
    STROKE_CONTRAST_COMPLEMENTARY_MIN = 1.5  # Meaningful difference
    STROKE_CONTRAST_COMPLEMENTARY_MAX = 5.0  # Not too extreme
    APERTURE_IDEAL_DIFF = 0.2  # Moderate difference = good
    
    @staticmethod
    def score_x_height_similarity(profile_a: dict, profile_b: dict) -> float:
        """
        Score x-height similarity.
        
        Uses unified optical_x_ratio which handles:
        - Actual x-height (lowercase)
        - Virtual x-height (ALL CAPS)
        - Estimated fallbacks
        
        Returns 0-1 (1 = perfect match)
        """
        metrics_a = profile_a.get('basic_metrics', {})
        metrics_b = profile_b.get('basic_metrics', {})
        
        # Use optical_x_ratio (unified metric)
        x_a = metrics_a.get('optical_x_ratio', 0.5)
        x_b = metrics_b.get('optical_x_ratio', 0.5)
        
        # Calculate similarity (closer = better)
        diff = abs(x_a - x_b)
        
        # Inverse exponential scoring
        # Similar ratios get high scores
        return max(0.0, 1.0 - (diff * 3.0))
    
    @staticmethod
    def calculate_width_variance_score(font_a: Dict, font_b: Dict) -> float:
        """
        Score width variance complementarity.
        
        Ideal: Moderate difference (0.05-0.15) for functional contrast.
        Too similar or too different = lower score.
        
        Args:
            font_a: Font profile with basic_metrics
            font_b: Font profile with basic_metrics
        
        Returns:
            Score 0.0-1.0
        """
        var_a = font_a.get('basic_metrics', {}).get('width_variance', 0)
        var_b = font_b.get('basic_metrics', {}).get('width_variance', 0)
        
        diff = abs(var_a - var_b)
        
        # Ideal range: 0.05 - 0.15
        if PairingScorer.WIDTH_VAR_IDEAL_MIN_DIFF <= diff <= PairingScorer.WIDTH_VAR_IDEAL_MAX_DIFF:
            score = 1.0
        elif diff < PairingScorer.WIDTH_VAR_IDEAL_MIN_DIFF:
            # Too similar
            score = diff / PairingScorer.WIDTH_VAR_IDEAL_MIN_DIFF  # Ramps from 0 to 1
        else:
            # Too different
            score = 0.5  # Penalty for extreme difference
        
        return score
    
    @staticmethod
    def calculate_perceptual_score(font_a: Dict, font_b: Dict) -> float:
        """
        Score perceptual similarity (moderate similarity ideal).
        
        Sweet spot: 0.90-0.98 similarity
        Too similar (>0.98) = likely same font/family
        Too different (<0.70) = may clash
        
        Args:
            font_a: Font profile with perceptual_layer
            font_b: Font profile with perceptual_layer
        
        Returns:
            Score 0.0-1.0
        """
        vec_a = font_a.get('perceptual_layer', {}).get('aggregated_embedding', [])
        vec_b = font_b.get('perceptual_layer', {}).get('aggregated_embedding', [])
        
        if not vec_a or not vec_b:
            return 0.5  # Neutral if missing data
        
        # Cosine similarity
        vec_a = np.array(vec_a)
        vec_b = np.array(vec_b)
        
        similarity = np.dot(vec_a, vec_b) / (np.linalg.norm(vec_a) * np.linalg.norm(vec_b))
        
        # Score based on ideal range
        if PairingScorer.PERCEPTUAL_SIM_MIN <= similarity <= PairingScorer.PERCEPTUAL_SIM_MAX:
            score = 1.0
        elif similarity > PairingScorer.PERCEPTUAL_SIM_MAX:
            # Too similar (penalize)
            score = 0.3
        elif similarity >= 0.70:
            # Moderate similarity
            score = 0.6
        else:
            # Very different
            score = 0.4
        
        return score
    
    @staticmethod
    def calculate_stroke_contrast_score(font_a: Dict, font_b: Dict) -> float:
        """
        Score stroke contrast complementarity (Phase 3).
        
        Ideal: Moderate difference for visual variety.
        Low + High contrast = excellent pairing.
        
        Args:
            font_a: Font profile with anatomy_metrics
            font_b: Font profile with anatomy_metrics
        
        Returns:
            Score 0.0-1.0
        """
        contrast_a = font_a.get('anatomy_metrics', {}).get('stroke_contrast', {}).get('mean_contrast', 1.0)
        contrast_b = font_b.get('anatomy_metrics', {}).get('stroke_contrast', {}).get('mean_contrast', 1.0)
        
        diff = abs(contrast_a - contrast_b)
        
        # Ideal: complementary contrast (geometric + transitional/modern)
        if PairingScorer.STROKE_CONTRAST_COMPLEMENTARY_MIN <= diff <= PairingScorer.STROKE_CONTRAST_COMPLEMENTARY_MAX:
            score = 1.0
        elif diff < PairingScorer.STROKE_CONTRAST_COMPLEMENTARY_MIN:
            # Too similar
            score = diff / PairingScorer.STROKE_CONTRAST_COMPLEMENTARY_MIN
        else:
            # Too extreme
            score = 0.6
        
        return score
    
    @staticmethod
    def calculate_aperture_score(font_a: Dict, font_b: Dict) -> float:
        """
        Score aperture openness compatibility (Phase 3).
        
        Moderate difference = good (variety without clash).
        
        Args:
            font_a: Font profile with anatomy_metrics
            font_b: Font profile with anatomy_metrics
        
        Returns:
            Score 0.0-1.0
        """
        aperture_a = font_a.get('anatomy_metrics', {}).get('aperture_openness', {}).get('mean_aperture', 0.5)
        aperture_b = font_b.get('anatomy_metrics', {}).get('aperture_openness', {}).get('mean_aperture', 0.5)
        
        diff = abs(aperture_a - aperture_b)
        
        # Ideal: some difference but not extreme
        if diff < PairingScorer.APERTURE_IDEAL_DIFF:
            score = 0.7  # Similar apertures = safe but less interesting
        elif diff < PairingScorer.APERTURE_IDEAL_DIFF * 2:
            score = 1.0  # Good variety
        else:
            score = 0.5  # Very different
        
        return score
    
    @staticmethod
    def calculate_category_bonus(font_a: Dict, font_b: Dict) -> float:
        """
        Bonus multiplier for category diversity.
        
        Args:
            font_a: Font profile with metadata_layer
            font_b: Font profile with metadata_layer
        
        Returns:
            Multiplier (1.0 or 1.2)
        """
        cat_a = font_a.get('metadata_layer', {}).get('google_fonts', {}).get('category')
        cat_b = font_b.get('metadata_layer', {}).get('google_fonts', {}).get('category')
        
        if cat_a and cat_b and cat_a != cat_b:
            return 1.2  # 20% bonus for serif+sans etc
        else:
            return 1.0  # No bonus
    
    @classmethod
    def calculate_pairing_score(cls, font_a: Dict, font_b: Dict) -> Tuple[float, Dict]:
        """
        Calculate overall pairing compatibility score.
        
        Updated for Phase 3 with anatomy metrics.
        
        Args:
            font_a: Complete font profile
            font_b: Complete font profile
        
        Returns:
            Tuple of (final_score, component_scores_dict)
        """
        # Calculate individual scores
        x_height_score = cls.calculate_x_height_score(font_a, font_b)
        width_score = cls.calculate_width_variance_score(font_a, font_b)
        perceptual_score = cls.calculate_perceptual_score(font_a, font_b)
        
        # Phase 3: Anatomy metrics
        stroke_score = cls.calculate_stroke_contrast_score(font_a, font_b)
        aperture_score = cls.calculate_aperture_score(font_a, font_b)
        
        category_bonus = cls.calculate_category_bonus(font_a, font_b)
        
        # Weighted combination (Phase 3 weights)
        base_score = (
            x_height_score * cls.WEIGHT_X_HEIGHT +
            width_score * cls.WEIGHT_WIDTH_VAR +
            stroke_score * cls.WEIGHT_STROKE_CONTRAST +
            aperture_score * cls.WEIGHT_APERTURE +
            perceptual_score * cls.WEIGHT_PERCEPTUAL +
            cls.WEIGHT_CATEGORY  # Base category weight
        )
        
        # Apply category bonus
        final_score = base_score * category_bonus
        
        # Component breakdown for explanation
        components = {
            'x_height_score': x_height_score,
            'width_variance_score': width_score,
            'stroke_contrast_score': stroke_score,
            'aperture_score': aperture_score,
            'perceptual_score': perceptual_score,
            'category_bonus': category_bonus,
            'base_score': base_score,
            'final_score': final_score,
        }
        
        return final_score, components


def test_scorer():
    """Test scorer with sample data."""
    
    # Mock font profiles (simplified)
    roboto = {
        'metadata': {'font_name': 'Roboto Regular'},
        'basic_metrics': {
            'x_height_ratio': 0.7431,
            'width_variance': 0.1431,
        },
        'metadata_layer': {
            'google_fonts': {'category': 'Sans Serif'}
        },
        'perceptual_layer': {
            'aggregated_embedding': np.random.rand(512).tolist()
        }
    }
    
    libre = {
        'metadata': {'font_name': 'Libre Baskerville'},
        'basic_metrics': {
            'x_height_ratio': 0.6883,
            'width_variance': 0.1931,
        },
        'metadata_layer': {
            'google_fonts': {'category': 'Serif'}
        },
        'perceptual_layer': {
            'aggregated_embedding': np.random.rand(512).tolist()
        }
    }
    
    print("Testing Pairing Scorer\n" + "="*60)
    
    score, components = PairingScorer.calculate_pairing_score(roboto, libre)
    
    print(f"\nPair: Roboto + Libre Baskerville")
    print(f"Final Score: {score:.4f}\n")
    print("Component Scores:")
    print(f"  X-height harmony:      {components['x_height_score']:.4f} (weight: 40%)")
    print(f"  Width complementarity: {components['width_variance_score']:.4f} (weight: 30%)")
    print(f"  Perceptual similarity: {components['perceptual_score']:.4f} (weight: 20%)")
    print(f"  Category bonus:        {components['category_bonus']:.2f}x")
    print(f"\n  Base score: {components['base_score']:.4f}")
    print(f"  After bonus: {components['final_score']:.4f}")


if __name__ == '__main__':
    test_scorer()
