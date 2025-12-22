# Font Classification Metrics - Algorithm Index

## Overview

This directory documents the 6 advanced font classification metrics implemented in FontAtlas. Each metric attempts to quantify specific typographic characteristics to enable automated font classification and pairing.

## Metrics Summary

| Metric | Status | Accuracy | Priority | File |
|--------|--------|----------|----------|------|
| **Terminal Sharpness** | ✓ Implemented | Good | Low | [terminal_sharpness.md](terminal_sharpness.md) |
| **Stress Angle** | ⚠️ Approximate | Poor | High | [stress_angle.md](stress_angle.md) |
| **Stroke Contrast** | ⚠️ Heuristic | Poor | Very High | [stroke_contrast.md](stroke_contrast.md) |
| **Aperture Openness** | ⚠️ Approximate | Fair | Medium | [aperture_openness.md](aperture_openness.md) |
| **Curvature Distribution** | ✓ Implemented | Good | Low | [curvature_distribution.md](curvature_distribution.md) |
| **Counter Density** | ❌ Flawed | Poor | Very High | [counter_density.md](counter_density.md) |

## Implementation Quality Assessment

### ✓ Good Quality (Production-ready)
1. **Terminal Sharpness**: Control point distance method is reasonable heuristic
2. **Curvature Distribution**: Curve-to-line ratio directly measures construction

### ⚠️ Needs Improvement (Functional but approximate)
3. **Aperture Openness**: Width-based proxy works but imprecise
4. **Stress Angle**: Bbox ratio is very rough approximation

### ❌ Critical Issues (Needs rework)
5. **Stroke Contrast**: Curve ratio doesn't measure actual stroke width
6. **Counter Density**: Inverse curve ratio has weak correlation

## Improvement Roadmap

### Phase 1: Critical Fixes (Immediate)
- [ ] **Stroke Contrast**: Implement proper stroke width scanning
- [ ] **Counter Density**: Use AreaPen or contour direction analysis

### Phase 2: Accuracy Improvements (#Medium-term)
- [ ] **Stress Angle**: Implement true stroke width measurement at multiple points
- [ ] **Aperture Openness**: Measure actual gap at opening

### Phase 3: Refinements (Long-term)
- [ ] Terminal Sharpness: Adaptive threshold based on UPM
- [ ] Curvature Distribution: Weight by segment length
- [ ] All: Validation dataset with human-labeled fonts

## Common Patterns

### Successful Approaches
- **Outline analysis**: Works well when metric maps directly to outline structure
- **Multiple character sampling**: Increases reliability
- **Bbox calculations**: Fast and useful for proportions

### Problematic Approaches
- **Indirect proxies**: When metric A is used to estimate unrelated metric B
- **Arbitrary thresholds**: Magic numbers that need per-font tuning
- **Heuristics without validation**: Assumptions not tested on real fonts

## Usage in Classification

Current classifier in [`font_classifier.py`](file:///Users/vietx/App%20Code/FontAtlas/src/metrics/font_classifier.py) uses:
- Width Variance → Monospace detection (✓ reliable)
- Stroke Contrast → Serif vs Sans (⚠️ needs improvement)
- Aperture + X-height → Display vs Text (⚠️ needs improvement)

**After improvements**, classification accuracy should increase significantly.

## Testing Recommendations

1. **Create validation dataset**: 
   - 20-30 well-known fonts
   - Human-labeled classifications
   - Expected metric ranges

2. **Measure accuracy**:
   - Compare calculated metrics to expected values
   - Identify systematic errors
   - Tune thresholds

3. **Edge case testing**:
   - Variable fonts
   - Non-Latin scripts
   - Decorative/display fonts
   - Monoline fonts

## Related Documentation

- [Classification Logic](../decisions/classification_logic.md) - How metrics combine
- [Font Anatomy](../research/font_anatomy.md) - Typography fundamentals
- [Database Schema](../decisions/database_schema.md) - How metrics are stored

---

**Created**: 2025-12-19  
**Last Updated**: 2025-12-19  
**Author**: AI Agent (Gemini)
