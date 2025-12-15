"""
Complementary Font Finder
==========================

Find and rank complementary font pairs using Qdrant + scoring logic.
"""

import json
from pathlib import Path
from typing import List, Dict
import sys

sys.path.insert(0, 'src')

from database.vector_db import FontAtlasVectorDB
from pairing.scorer import PairingScorer


class ComplementaryFinder:
    """Find complementary font pairs."""
    
    def __init__(self, db: FontAtlasVectorDB = None):
        """
        Initialize finder.
        
        Args:
            db: Vector database client (creates new if None)
        """
        self.db = db or FontAtlasVectorDB()
        self.scorer = PairingScorer()
    
    def load_font_profile(self, font_path: str) -> Dict:
        """Load complete font profile from file."""
        with open(font_path, 'r', encoding='utf-8') as f:
            return json.load(f)
    
    def find_complementary_pairs(
        self,
        query_font_path: str,
        limit: int = 5,
        min_score: float = 0.5
    ) -> List[Dict]:
        """
        Find complementary fonts for a query font.
        
        Args:
            query_font_path: Path to query font's .complete.json
            limit: Max results to return
            min_score: Minimum pairing score threshold
        
        Returns:
            List of dicts with font info and scores
        """
        # Load query font profile
        query_profile = self.load_font_profile(query_font_path)
        query_vector = query_profile['perceptual_layer']['aggregated_embedding']
        query_name = query_profile['metadata']['font_name']
        
        print(f"\n{'='*70}")
        print(f"Finding complementary pairs for: {query_name}")
        print(f"{'='*70}\n")
        
        # Get perceptually similar fonts from Qdrant (wider net)
        similar_fonts = self.db.search_similar(
            query_vector=query_vector,
            limit=limit * 2  # Get more candidates than needed
        )
        
        print(f"Retrieved {len(similar_fonts)} candidate fonts from Qdrant\n")
        
        # Score each candidate using pairing algorithm
        scored_pairs = []
        
        for candidate in similar_fonts:
            # Skip self-match
            if candidate['font_name'] == query_name:
                continue
            
            # Reconstruct candidate profile from payload
            candidate_profile = {
                'metadata': {'font_name': candidate['font_name']},
                'basic_metrics': {
                    'x_height_ratio': candidate['payload'].get('x_height_ratio'),
                    'width_variance': candidate['payload'].get('width_variance'),
                    'width_mean': candidate['payload'].get('width_mean'),
                },
                'unicode_coverage': {
                    'total_glyphs': candidate['payload'].get('total_glyphs'),
                    'supports_vietnamese': candidate['payload'].get('supports_vietnamese'),
                },
                'metadata_layer': {
                    'google_fonts': {
                        'category': candidate['payload'].get('category'),
                        'designers': candidate['payload'].get('designers', []),
                        'popularity_rank': candidate['payload'].get('popularity_rank'),
                    }
                },
                # Note: We don't have full perceptual vector in payload,
                # but we have the similarity score from Qdrant
                'perceptual_similarity': candidate['score']
            }
            
            # Calculate pairing score
            pairing_score, components = self.scorer.calculate_pairing_score(
                query_profile,
                candidate_profile
            )
            
            # Filter by minimum score
            if pairing_score >= min_score:
                scored_pairs.append({
                    'font_name': candidate['font_name'],
                    'pairing_score': pairing_score,
                    'components': components,
                    'profile': candidate_profile,
                    'qdrant_similarity': candidate['score'],
                })
        
        # Sort by pairing score (descending)
        scored_pairs.sort(key=lambda x: x['pairing_score'], reverse=True)
        
        # Return top N
        return scored_pairs[:limit]


def main():
    """CLI for finding complementary pairs."""
    import argparse
    from pairing.explainer import PairingExplainer
    
    parser = argparse.ArgumentParser(description='Find complementary font pairs')
    parser.add_argument('font', help='Path to .complete.json font profile')
    parser.add_argument('--limit', type=int, default=5, help='Number of results')
    parser.add_argument('--min-score', type=float, default=0.5, help='Minimum pairing score')
    parser.add_argument('--explain', action='store_true', help='Show detailed explanations')
    
    args = parser.parse_args()
    
    # Initialize finder
    finder = ComplementaryFinder()
    
    # Load query font profile
    query_profile = finder.load_font_profile(args.font)
    query_name = query_profile['metadata']['font_name']
    
    print(f"\n{'╔'+'═'*68+'╗'}")
    print(f"║ {'FontAtlas - Complementary Font Finder':^68} ║")
    print(f"{'╚'+'═'*68+'╝'}\n")
    
    print(f"Query Font: {query_name}")
    print(f"Category: {query_profile.get('metadata_layer', {}).get('google_fonts', {}).get('category', 'Unknown')}")
    print(f"X-height: {query_profile['basic_metrics']['x_height_ratio']:.3f} | "
          f"Width variance: {query_profile['basic_metrics']['width_variance']:.3f}\n")
    
    # Find pairs
    pairs = finder.find_complementary_pairs(
        query_font_path=args.font,
        limit=args.limit,
        min_score=args.min_score
    )
    
    if not pairs:
        print(f"{'─'*70}")
        print("No complementary pairs found matching criteria.")
        return
    
    # Display results
    print(f"{'─'*70}\n")
    print(f"TOP COMPLEMENTARY PAIRS:\n")
    
    explainer = PairingExplainer()
    
    for i, pair in enumerate(pairs, 1):
        if args.explain:
            # Full explanation
            explanation = explainer.generate_explanation(
                font_a=query_profile,
                font_b=pair['profile'],
                pairing_score=pair['pairing_score'],
                components=pair['components']
            )
            print(f"{i}. {explanation}")
        else:
            # Compact summary
            print(f"{i}. {pair['font_name']} (Score: {pair['pairing_score']:.4f}) ⭐")
            print(f"   Qdrant similarity: {pair['qdrant_similarity']:.4f}")
            print(f"   Component scores: X-height {pair['components']['x_height_score']:.2f} | "
                  f"Width {pair['components']['width_variance_score']:.2f} | "
                  f"Perceptual {pair['components']['perceptual_score']:.2f}")
            print()



if __name__ == '__main__':
    main()
