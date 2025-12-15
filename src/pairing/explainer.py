"""
Pairing Explanation Generator
==============================

Generate human-readable explanations for font pairings.
Per spec: Explain WHY fonts pair well, not just THAT they do.
"""

from typing import Dict


class PairingExplainer:
    """Generate structural explanations for font pairs."""
    
    @staticmethod
    def interpret_x_height_diff(diff: float) -> str:
        """Interpret x-height difference."""
        if diff < 0.05:
            return "VERY SIMILAR - Shared baseline rhythm, minimal vertical contrast"
        elif diff < 0.10:
            return "SIMILAR - Compatible vertical proportions, subtle hierarchy potential"
        elif diff < 0.15:
            return "MODERATE - Noticeable but harmonious vertical difference"
        else:
            return "DISTINCT - Strong vertical contrast, may create visual tension"
    
    @staticmethod
    def interpret_width_variance(var_a: float, var_b: float) -> str:
        """Interpret width variance relationship."""
        interpretations = []
        
        # Classify font A
        if var_a < 0.05:
            interpretations.append("Font A: Geometric/uniform character widths (modern, clean feel)")
        elif var_a > 0.15:
            interpretations.append("Font A: Humanist/varied character widths (organic, traditional feel)")
        else:
            interpretations.append("Font A: Moderate width variation (versatile)")
        
        # Classify font B
        if var_b < 0.05:
            interpretations.append("Font B: Geometric/uniform character widths (modern, clean feel)")
        elif var_b > 0.15:
            interpretations.append("Font B: Humanist/varied character widths (organic, traditional feel)")
        else:
            interpretations.append("Font B: Moderate width variation (versatile)")
        
        # Relationship
        diff = abs(var_a - var_b)
        if 0.05 <= diff <= 0.15:
            interpretations.append("→ COMPLEMENTARY: Ideal contrast for functional pairing (one display, one body)")
        elif diff < 0.05:
            interpretations.append("→ SIMILAR: Both fonts share similar rhythm patterns")
        else:
            interpretations.append("→ CONTRASTING: Strong stylistic difference")
        
        return "\n   ".join(interpretations)
    
    @staticmethod
    def interpret_perceptual_similarity(similarity: float) -> str:
        """Interpret perceptual embedding similarity."""
        if similarity > 0.98:
            return "VERY SIMILAR - Likely same family or style, low visual variety"
        elif similarity > 0.90:
            return "HARMONIOUS - Compatible visual texture and feel"
        elif similarity > 0.70:
            return "COMPLEMENTARY - Interesting visual contrast while maintaining coherence"
        else:
            return "VERY DIFFERENT - Strong perceptual contrast, use carefully"
    
    @staticmethod
    def recommend_usage(font_a_name: str, font_b_name: str, 
                       width_var_a: float, width_var_b: float,
                       category_a: str, category_b: str) -> str:
        """Recommend usage based on structural characteristics."""
        recommendations = []
        
        # Determine roles based on width variance
        if width_var_a < width_var_b:
            # Font A more uniform = better for headings
            recommendations.append(f"• {font_a_name}: Headings, display, UI elements, short text")
            recommendations.append(f"• {font_b_name}: Body copy, paragraphs, long-form content")
            recommendations.append(f"  Reasoning: {font_a_name}'s geometric uniformity creates strong hierarchy")
        else:
            # Font B more uniform = better for headings
            recommendations.append(f"• {font_b_name}: Headings, display, UI elements, short text")
            recommendations.append(f"• {font_a_name}: Body copy, paragraphs, long-form content")
            recommendations.append(f"  Reasoning: {font_b_name}'s uniformity provides clear visual anchors")
        
        # Category-based notes
        if category_a and category_b and category_a != category_b:
            recommendations.append(f"  Note: Serif + Sans pairing provides traditional hierarchy and readability")
        
        return "\n".join(recommendations)
    
    @classmethod
    def generate_explanation(
        cls,
        font_a: Dict,
        font_b: Dict,
        pairing_score: float,
        components: Dict
    ) -> str:
        """
        Generate complete pairing explanation.
        
        Args:
            font_a: Query font profile
            font_b: Candidate font profile
            pairing_score: Overall pairing score
            components: Component scores dict
        
        Returns:
            Formatted explanation string
        """
        # Extract data
        font_a_name = font_a['metadata']['font_name']
        font_b_name = font_b['metadata']['font_name']
        
        x_height_a = font_a.get('basic_metrics', {}).get('x_height_ratio', 0)
        x_height_b = font_b.get('basic_metrics', {}).get('x_height_ratio', 0)
        x_height_diff = abs(x_height_a - x_height_b)
        
        width_var_a = font_a.get('basic_metrics', {}).get('width_variance', 0)
        width_var_b = font_b.get('basic_metrics', {}).get('width_variance', 0)
        
        category_a = font_a.get('metadata_layer', {}).get('google_fonts', {}).get('category')
        category_b = font_b.get('metadata_layer', {}).get('google_fonts', {}).get('category')
        
        perceptual_sim = font_b.get('perceptual_similarity', 0.9)  # From Qdrant
        
        # Build explanation
        explanation = f"""
{'═'*70}
{font_b_name} (Pairing Score: {pairing_score:.4f})
{'═'*70}

🔍 STRUCTURAL COMPATIBILITY ANALYSIS

X-Height Harmony:
   {font_a_name}: {x_height_a:.4f}
   {font_b_name}: {x_height_b:.4f}
   Difference: {x_height_diff:.4f} ({x_height_diff/x_height_a*100:.1f}%)
   
   → {cls.interpret_x_height_diff(x_height_diff)}
   Score: {components['x_height_score']:.4f}/1.00 (weight: 40%)

Width Variance:
   {font_a_name}: {width_var_a:.4f}
   {font_b_name}: {width_var_b:.4f}
   
   {cls.interpret_width_variance(width_var_a, width_var_b)}
   Score: {components['width_variance_score']:.4f}/1.00 (weight: 30%)

🎨 PERCEPTUAL RELATIONSHIP

Similarity: {perceptual_sim:.4f}
→ {cls.interpret_perceptual_similarity(perceptual_sim)}
Score: {components['perceptual_score']:.4f}/1.00 (weight: 20%)

Category: {category_a or 'Unknown'} + {category_b or 'Unknown'}
Bonus: {components['category_bonus']:.2f}x

{'─'*70}

📋 RECOMMENDED USAGE

{cls.recommend_usage(font_a_name, font_b_name, width_var_a, width_var_b, category_a, category_b)}

{'─'*70}

FINAL PAIRING SCORE: {pairing_score:.4f}/1.00
"""
        
        return explanation


def test_explainer():
    """Test explanation generator."""
    
    # Mock data
    roboto = {
        'metadata': {'font_name': 'Roboto Regular'},
        'basic_metrics': {
            'x_height_ratio': 0.7431,
            'width_variance': 0.1431,
        },
        'metadata_layer': {
            'google_fonts': {'category': 'Sans Serif'}
        }
    }
    
    libre = {
        'metadata': {'font_name': 'Libre Baskerville Regular'},
        'basic_metrics': {
            'x_height_ratio': 0.6883,
            'width_variance': 0.1931,
        },
        'metadata_layer': {
            'google_fonts': {'category': 'Serif'}
        },
        'perceptual_similarity': 0.9867
    }
    
    components = {
        'x_height_score': 0.6345,
        'width_variance_score': 1.0000,
        'perceptual_score': 0.5000,
        'category_bonus': 1.20,
    }
    
    explanation = PairingExplainer.generate_explanation(
        roboto, libre, 0.7538, components
    )
    
    print(explanation)


if __name__ == '__main__':
    test_explainer()
