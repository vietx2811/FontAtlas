"""
Pairing Router
===============

Endpoints for font pair finding and comparison.
"""

from fastapi import APIRouter, HTTPException, Query
from pathlib import Path
import json
from typing import Optional

from api.models import PairingRequest, PairingResponse, PairingResult, PairingComponentScores
from pairing.finder import ComplementaryFinder

router = APIRouter()

# Default directory
DEFAULT_FONT_DIR = "../fonts/samples"


@router.post("/find", response_model=PairingResponse)
async def find_pairs(request: PairingRequest, directory: Optional[str] = Query(DEFAULT_FONT_DIR)):
    """
    Find complementary font pairs for a given font.
    
    Uses Qdrant vector search + multi-factor pairing scorer.
    """
    # Load query font profile
    profile_path = Path(directory) / f"{request.font_id}.complete.json"
    
    if not profile_path.exists():
        raise HTTPException(status_code=404, detail=f"Font '{request.font_id}' not found in {directory}")
    
    # Find pairs
    try:
        finder = ComplementaryFinder()
        pairs = finder.find_complementary_pairs(
            str(profile_path),
            limit=request.limit
        )
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Pairing search failed: {str(e)}")
    
    # Format response
    results = []
    for pair in pairs:
        components = PairingComponentScores(
            x_height_score=pair['component_scores']['x_height_score'],
            width_variance_score=pair['component_scores']['width_variance_score'],
            stroke_contrast_score=pair['component_scores']['stroke_contrast_score'],
            aperture_score=pair['component_scores']['aperture_score'],
            perceptual_score=pair['component_scores']['perceptual_score'],
            category_bonus=pair['component_scores']['category_bonus']
        )
        
        results.append(PairingResult(
            font_id=pair.get('font_id', 'unknown'),
            font_name=pair['font_name'],
            score=pair['pairing_score'],
            qdrant_similarity=pair['similarity_score'],
            components=components,
            explanation=None  # TODO: Add explanation if requested
        ))
    
    # Get query font name
    with open(profile_path, 'r') as f:
        profile = json.load(f)
    query_name = profile.get('metadata', {}).get('font_name', 'Unknown')
    
    return PairingResponse(
        query_font=query_name,
        pairs=results
    )


@router.get("/compare")
async def compare_fonts(font_a: str, font_b: str, directory: Optional[str] = Query(DEFAULT_FONT_DIR)):
    """
    Compare two specific fonts directly.
    
    Returns compatibility score and component breakdown.
    """
    from pairing.scorer import PairingScorer
    
    # Load both profiles
    profile_a_path = Path(directory) / f"{font_a}.complete.json"
    profile_b_path = Path(directory) / f"{font_b}.complete.json"
    
    if not profile_a_path.exists():
        raise HTTPException(status_code=404, detail=f"Font '{font_a}' not found in {directory}")
    if not profile_b_path.exists():
        raise HTTPException(status_code=404, detail=f"Font '{font_b}' not found in {directory}")
    
    with open(profile_a_path, 'r') as f:
        profile_a = json.load(f)
    with open(profile_b_path, 'r') as f:
        profile_b = json.load(f)
    
    # Calculate score
    score, components = PairingScorer.calculate_pairing_score(profile_a, profile_b)
    
    return {
        "font_a": profile_a.get('metadata', {}).get('font_name', 'Unknown'),
        "font_b": profile_b.get('metadata', {}).get('font_name', 'Unknown'),
        "score": score,
        "components": components,
        "recommendation": _generate_recommendation(score)
    }


def _generate_recommendation(score: float) -> str:
    """Generate simple recommendation based on score."""
    if score >= 0.8:
        return "Excellent pairing - highly compatible"
    elif score >= 0.7:
        return "Good pairing - recommended for heading + body"
    elif score >= 0.6:
        return "Moderate pairing - use with caution"
    else:
        return "Weak pairing - consider alternatives"
