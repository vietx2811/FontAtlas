"""Custom Anatomy Metrics Package

This package provides custom typography-specific geometry analysis:
- Stroke contrast measurement
- Aperture detection
- Terminal classification
- Stress angle analysis
"""

from .stroke_contrast import StrokeContrastAnalyzer
from .aperture_detector import ApertureDetector

__all__ = ['StrokeContrastAnalyzer', 'ApertureDetector']
