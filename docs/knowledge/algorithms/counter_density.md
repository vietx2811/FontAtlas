# Counter Density

## Context

Counter là khoảng trắng bên trong characters (như internal của 'o', 'p', 'g', 'b', 'd'). Counter density đo mức độ "spacious" vs "tight" của counters:
- **Tight counters**: Stroke dày → counter nhỏ → condensed feel
- **Spacious counters**: Stroke mỏng → counter lớn → open, airy feel

Ảnh hưởng đến:
- Legibility (spacious counters dễ đọc hơn)
- Density/texture của text block
- Style (condensed vs extended)

## Research

**Ideal measurement:**
```
counter_area / bbox_area
```
- Counter area: White space inside glyph
- Bbox area: Total bounding box

**Challenge:** Calculate counter area from outline
- Need to identify closed regions (counters)
- Measure area of those regions
- Complex with self-intersecting paths

**Heuristic approach** (Chosen):
- Use **curve ratio as inverse proxy**
- More curves → thicker strokes (complex outline) → smaller counters
- Fewer curves → thinner strokes (simple outline) → larger counters

## Decision

Sử dụng **inverse of curve ratio** như estimate:

**Logic:**
- Geometric fonts (low curves) → thin strokes → spacious counters
- Complex fonts (high curves) → thick/variable strokes → tight counters

**Formula:**
```python
curve_ratio = curves / (lines + curves)
density = 1.0 - curve_ratio * 0.5  # Inverse relationship
```

**Result:**
- Low curve ratio → High density score (large counters)
- High curve ratio → Low density score (small counters)

## Implementation Notes

**File**: [`src/parsers/font_parser.py:699`](file:///Users/vietx/App%20Code/FontAtlas/src/parsers/font_parser.py#L699-L760)

**Algorithm:**
1. Sample 'o', 'p', 'g', 'b', 'd'
2. For each, count lines and curves
3. Calculate curve ratio
4. Invert: `1.0 - curve_ratio * 0.5`
5. Return mean

**Return value**: 0.0 (tight/small counters) to 1.0 (spacious/large counters)

**Fallback**: 0.5 on errors

## Trade-offs

### Pros
- ✓ Simple to calculate
- ✓ Uses existing outline data

### Cons
- ✗ **VERY indirect** - curve count has weak correlation with counter size
- ✗ Confounds stroke complexity with counter space
- ✗ Doesn't actually measure area
- ✗ Arbitrary inverse formula

## Known Issues

**This is a poor metric.** 

The assumption that curves = thick strokes = small counters is questionable:
- A font can have many curves AND large counters
- Stroke thickness is independent of outline complexity

**Examples that may fail:**
- Thin, curvy script font → Many curves but large counters
- Bold geometric font → Few curves but small counters

## Future Improvements

### Priority: Very High (Current approach is fundamentally flawed)

**Implement proper area calculation:**

```python
from fontTools.pens.areaPen import AreaPen

def calculate_counter_density(glyph):
    # Get total glyph area (including counters)
    area_pen = AreaPen()
    glyph.draw(area_pen)
    total_area = abs(area_pen.value)
    
    # Get bbox area
    bbox_area = (xMax - xMin) * (yMax - yMin)
    
    # Counter area ≈ bbox_area - total_area
    # (This is approximate, better methods exist)
    counter_area = bbox_area - total_area
    
    # Density = counter size relative to bbox
    density = counter_area / bbox_area
    
    return density
```

**OR: Use contour direction analysis**
- Outer contour = clockwise
- Inner contour (counter) = counter-clockwise
- Calculate area of inner contours only

**fontTools.pens.areaPen** may already support this!

## Related Knowledge

- [Aperture Openness](aperture_openness.md) - Related to counter analysis
- [Stroke Contrast](stroke_contrast.md) - Affects counter size
- [Classification Logic](../decisions/classification_logic.md)

## Expected Values

**After proper implementation:**
- Condensed fonts: 0.2-0.4 (tight)
- Normal fonts: 0.4-0.6 (moderate)
- Extended fonts: 0.6-0.8 (spacious)

**Current implementation** will NOT match these.

---

**Created**: 2025-12-19  
**Last Updated**: 2025-12-19  
**Author**: AI Agent (Gemini)  
**Status**: Implemented (Needs Major Rework)  
**Priority**: Very high improvement priority
