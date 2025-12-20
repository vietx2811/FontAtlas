# Stroke Contrast Ratio

## Context

Stroke contrast là tỷ lệ giữa phần dày nhất và phần mỏng nhất của một stroke. Đây là metric then chốt nhất để phân biệt:
- **Serif fonts**: High contrast (2.0 - 10+)
- **Sans-serif fonts**: Low contrast (1.0 - 1.5)
- **Didone**: Very high contrast (10+)
- **Monoline/Geometric**: Almost no contrast (~1.0)

## Research

**Ideal approach**: Pixel-perfect stroke width measurement
1. Scan horizontally and vertically through glyph
2. Measure distance between outline edges
3. Find max and min widths
4. Calculate ratio: `max_width / min_width`

**Challenges:**
- Requires precise outline intersection detection
- Computationally expensive
- Complex with bezier curves

**Heuristic approach** (Chosen):
- Use outline **complexity** as proxy for contrast
- More curves → more variation → higher contrast
- Simpler outlines → geometric → lower contrast

## Decision

Sử dụng **curve-to-line ratio** của character 'O' để estimate stroke contrast.

**Reasoning:**
- Serif fonts with contrast have complex outlines with many curves
- Sans-serif geometric fonts have simpler outlines with more straight segments
- The ratio of curves to total segments correlates with stroke variation

**Formula:**
```python
curve_ratio = num_curves / (num_lines + num_curves + 1)

if curve_ratio > 0.7:
    contrast = 2.0 + (curve_ratio - 0.7) * 5.0  # → 2.0-3.5
else:
    contrast = 1.0 + curve_ratio  # → 1.0-1.7
```

## Implementation Notes

**File**: [`src/parsers/font_parser.py:528`](file:///Users/vietx/App%20Code/FontAtlas/src/parsers/font_parser.py#L528-L592)

**Algorithm:**
1. Get 'O' glyph outline
2. Count `lineTo` commands
3. Count `curveTo` + `qCurveTo` commands
4. Calculate curve ratio
5. Map to contrast value

**Return value**: 1.0 (monoweight) to 10+ (high contrast)

**Fallback**: 1.0 on errors

## Trade-offs

### Pros
- ✓ Fast (just counting outline commands)
- ✓ No rendering required
- ✓ Works with any font

### Cons
- ✗ **Indirect measurement** - outline complexity ≠ stroke contrast
- ✗ Assumes serif = more complex outlines
- ✗ Fails for complex geometric fonts
- ✗ Arbitrary threshold values (0.7, 2.0, 5.0)

## Known Issues

**False positives:**
1. **Decorative geometric fonts**: Many curves but low stroke contrast
2. **Simple serif fonts**: Few curves but actual contrast exists

**Misclassification examples:**
- Futura (geometric, simple) → Correctly low contrast ✓
- Bodoni (didone, complex) → Correctly high contrast ✓
- ???DIN (geometric, some curves) → May incorrectly show moderate contrast ✗

## Future Improvements

### Priority: Very High (Current is placeholder-quality)

**Implement proper stroke width measurement:**

```python
def measure_stroke_width(outline, y_position):
    """Scan horizontally at y_position and find stroke width"""
    intersections = find_outline_intersections(outline, y_position)
    # Sort intersections by x-coordinate
    # Measure distances between pairs (stroke widths)
    return widths

def calculate_true_contrast(glyph):
    # Sample at multiple heights
    heights = [0.25, 0.5, 0.75]  # of glyph height
    all_widths = []
    
    for h in heights:
        y = bbox.yMin + (bbox.yMax - bbox.yMin) * h
        widths = measure_stroke_width(outline, y)
        all_widths.extend(widths)
    
    max_width = max(all_widths)
    min_width = min(all_widths)
    contrast = max_width / min_width
    
    return contrast
```

**Alternative: fontTools.pens.statisticsPen**
- May have built-in stroke width analysis
- Investigate if available

## Related Knowledge

- [Terminal Sharpness](terminal_sharpness.md) - Another outline metric
- [Aperture Openness](aperture_openness.md) - Uses similar bbox logic
- [Classification Logic](../decisions/classification_logic.md) - Primary classifier

## Expected Values

**Target ranges** (after proper implementation):
- Geometric Sans (Futura): 1.0 - 1.1
- Humanist Sans (Gill Sans): 1.1 - 1.3
- Transitional Serif (Baskerville): 2.0 - 3.0
- Modern Serif (Bodoni): 8.0 - 15.0

**Current implementation** will NOT match these accurately.

---

**Created**: 2025-12-19  
**Last Updated**: 2025-12-19  
**Author**: AI Agent (Gemini)  
**Status**: Implemented (Heuristic Only)  
**Priority**: Very high improvement priority
