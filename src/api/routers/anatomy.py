"""
Anatomy Router
===============

Endpoints for detailed anatomy metrics (stroke contrast, aperture).
"""

from fastapi import APIRouter, HTTPException, Query
from pathlib import Path
import json
from typing import Optional

from api.models import StrokeContrastResponse, StrokeContrastGlyph, ApertureResponse, ApertureGlyph

router = APIRouter()

# Default directory
DEFAULT_FONT_DIR = "../fonts/samples"


@router.get("/stroke-contrast/{font_id}", response_model=StrokeContrastResponse)
async def get_stroke_contrast(font_id: str, directory: Optional[str] = Query(DEFAULT_FONT_DIR)):
    """
    Get detailed stroke contrast analysis for a font.
    
    Includes per-glyph breakdown and classification.
    """
    profile_path = Path(directory) / f"{font_id}.complete.json"
    
    if not profile_path.exists():
        raise HTTPException(status_code=404, detail=f"Font '{font_id}' not found in {directory}")
    
    with open(profile_path, 'r') as f:
        profile = json.load(f)
    
    font_name = profile.get('metadata', {}).get('font_name', 'Unknown')
    anatomy = profile.get('anatomy_metrics', {})
    stroke_data = anatomy.get('stroke_contrast', {})
    
    if not stroke_data or 'error' in stroke_data:
        raise HTTPException(status_code=404, detail="Stroke contrast data not available")
    
    # Format glyph data
    by_glyph = {}
    for glyph_name, glyph_data in stroke_data.get('by_glyph', {}).items():
        if 'error' not in glyph_data:
            by_glyph[glyph_name] = StrokeContrastGlyph(
                contrast_ratio=glyph_data['contrast_ratio'],
                max_thickness=glyph_data['max_thickness'],
                min_thickness=glyph_data['min_thickness'],
                mean_thickness=glyph_data['mean_thickness'],
                glyph=glyph_data['glyph']
            )
    
    return StrokeContrastResponse(
        font=font_name,
        mean_contrast=stroke_data.get('mean_contrast', 1.0),
        classification=stroke_data.get('classification', 'unknown'),
        by_glyph=by_glyph
    )


@router.get("/aperture/{font_id}", response_model=ApertureResponse)
async def get_aperture(font_id: str, directory: Optional[str] = Query(DEFAULT_FONT_DIR)):
    """
    Get aperture openness analysis for a font.
    
    Includes per-glyph breakdown with detection methods.
    """
    profile_path = Path(directory) / f"{font_id}.complete.json"
    
    if not profile_path.exists():
        raise HTTPException(status_code=404, detail=f"Font '{font_id}' not found in {directory}")
    
    with open(profile_path, 'r') as f:
        profile = json.load(f)
    
    font_name = profile.get('metadata', {}).get('font_name', 'Unknown')
    anatomy = profile.get('anatomy_metrics', {})
    aperture_data = anatomy.get('aperture_openness', {})
    
    if not aperture_data or 'error' in aperture_data:
        raise HTTPException(status_code=404, detail="Aperture data not available")
    
    # Format glyph data
    by_glyph = {}
    for glyph_name, glyph_data in aperture_data.get('by_glyph', {}).items():
        if 'error' not in glyph_data and glyph_data.get('has_aperture'):
            by_glyph[glyph_name] = ApertureGlyph(
                score=glyph_data['normalized_score'],
                width=glyph_data['opening_width'],
                detection_method=glyph_data.get('detection_method', 'unknown'),
                glyph=glyph_name
            )
    
    return ApertureResponse(
        font=font_name,
        mean_aperture=aperture_data.get('mean_aperture', 0.5),
        classification=aperture_data.get('classification', 'unknown'),
        by_glyph=by_glyph
    )
