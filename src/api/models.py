"""
API Data Models
================

Pydantic models for request/response validation.
"""

from pydantic import BaseModel, Field
from typing import Dict, List, Optional


class FontSummary(BaseModel):
    """Summary information for a font."""
    id: str = Field(..., description="Font identifier (filename without extension)")
    name: str = Field(..., description="Display name")
    category: str = Field(..., description="Font category (Serif, Sans Serif, etc.)")
    x_height: float = Field(..., description="X-height ratio")
    width_variance: float = Field(..., description="Width variance metric")
    stroke_contrast: float = Field(..., description="Mean stroke contrast ratio")
    aperture: float = Field(..., description="Mean aperture openness score")


class FontListResponse(BaseModel):
    """Response for font listing."""
    fonts: List[FontSummary]
    count: int


class PairingComponentScores(BaseModel):
    """Individual component scores for a pairing."""
    x_height_score: float
    width_variance_score: float
    stroke_contrast_score: float
    aperture_score: float
    perceptual_score: float
    category_bonus: float


class PairingResult(BaseModel):
    """Result for a single font pairing."""
    font_id: str
    font_name: str
    score: float
    qdrant_similarity: float
    components: PairingComponentScores
    explanation: Optional[str] = None


class PairingRequest(BaseModel):
    """Request to find complementary pairs."""
    font_id: str = Field(..., description="Font ID to find pairs for")
    limit: int = Field(3, ge=1, le=10, description="Number of results")
    explain: bool = Field(False, description="Include detailed explanations")


class PairingResponse(BaseModel):
    """Response with pairing results."""
    query_font: str
    pairs: List[PairingResult]


class StrokeContrastGlyph(BaseModel):
    """Stroke contrast data for a single glyph."""
    contrast_ratio: float
    max_thickness: float
    min_thickness: float
    mean_thickness: float
    glyph: str


class StrokeContrastResponse(BaseModel):
    """Stroke contrast analysis response."""
    font: str
    mean_contrast: float
    classification: str
    by_glyph: Dict[str, StrokeContrastGlyph]


class ApertureGlyph(BaseModel):
    """Aperture data for a single glyph."""
    score: float
    width: float
    detection_method: str
    glyph: str


class ApertureResponse(BaseModel):
    """Aperture openness analysis response."""
    font: str
    mean_aperture: float
    classification: str
    by_glyph: Dict[str, ApertureGlyph]
