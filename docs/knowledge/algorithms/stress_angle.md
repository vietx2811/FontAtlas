# Stress Angle Measurement

## Context

Stress angle là góc nghiêng của phần dày nhất trong rounded strokes (như 'o', 'e', 'b'). Đây là đặc điểm then chốt để phân biệt:
- **Old Style / Humanist**: Stress angle nghiêng (~15-30°) - bắt chước calligraphy
- **Transitional**: Stress angle gần vertical (~5-15°)
- **Modern / Didone**: Stress angle hoàn toàn vertical (0°)
- **Geometric Sans**: Stress có thể horizontal hoặc không có

## Research

**Traditional method** (Typography theory):
- Vẽ đường nối 2 điểm dày nhất trên counter của 'o'
- Đo góc của đường này với trục vertical
- Requires precise stroke width measurement

**Challenges:**
1. Stroke width measurement phức tạp với outlines
2. Cần scan toàn bộ glyph để tìm điểm dày nhất
3. Computational expensive

**Simplified approach** (Chosen):
- Dùng **aspect ratio** của bounding box
- Wide bbox → horizontal stress
- Tall bbox → vertical stress

## Decision

Sử dụng **bbox aspect ratio heuristic** của character 'o' để approximate stress angle.

**Formula:**
```python
aspect_ratio = width / height
angle = min(90.0, 30.0 + (aspect_ratio - 0.7) * 40.0)
angle = max(0.0, angle)
```

**Mapping:**
- aspect_ratio = 1.0 (square) → ~42° angle
- aspect_ratio > 1.0 (wider) → higher angle (more horizontal)
- aspect_ratio < 1.0 (taller) → lower angle (more vertical)

## Implementation Notes

**File**: [`src/parsers/font_parser.py:476`](file:///Users/vietx/App%20Code/FontAtlas/src/parsers/font_parser.py#L476-L526)

**Algorithm:**
1. Get glyph 'o' from character map
2. Use `BoundsPen` to calculate bbox
3. Calculate `width / height` ratio
4. Map ratio to angle (0° - 90°)

**Return value**: Angle in degrees (0° = vertical, 90° = horizontal)

**Fallback**: 0.0° on any errors

## Trade-offs

### Pros
- ✓ Very fast (just bbox calculation)
- ✓ No stroke width analysis needed
- ✓ Works for any font with 'o' glyph

### Cons
- ✗ **VERY approximate** - not true stress angle
- ✗ Bbox shape ≠ stroke thickness distribution
- ✗ Arbitrary formula (30.0 + ... * 40.0)
- ✗ Only samples one character ('o')
- ✗ Doesn't account for counter shape

## Known Limitations

**This is the least accurate metric we have.**

The bbox aspect ratio is influenced by:
- Counter width (not just stress)
- Overshoot (extends beyond baseline/x-height)
- Glyph proportions (independent of stress)

**Real stress angle requires:**
- Stroke width measurement at multiple points
- Finding thickest point on left and right sides
- Calculating angle between them

## Future Improvements

### Priority: High (Current implementation is rough approximation)

1. **Proper stroke width analysis**:
   ```python
   # Sample vertical cross-sections
   for y in range(yMin, yMax, step):
       horizontal_scan(y)
       measure_stroke_width()
   
   # Find thickest points
   left_thick = find_max_width_left()
   right_thick = find_max_width_right()
   
   # Calculate angle
   angle = atan2(left_thick.y - right_thick.y, 
                 left_thick.x - right_thick.x)
   ```

2. **Multiple character sampling**: 'o', 'e', 'b', 'd'

3. **Counter shape analysis**: Analyze internal counter, not bbox

4. **Reference dataset**: Measure known fonts to calibrate formula

## Related Knowledge

- [Stroke Contrast](stroke_contrast.md) - Related stroke measurement
- [Counter Density](counter_density.md) - Also uses internal shapes
- [Classification Logic](../decisions/classification_logic.md)

## Expected Values

**Theoretical** (not yet validated):
- Garamond (Old Style): 20-30°
- Baskerville (Transitional): 10-20°
- Bodoni (Modern): 0-5°
- Helvetica (Sans): Variable

**Current implementation** may not match these closely due to approximation.

---

**Created**: 2025-12-19  
**Last Updated**: 2025-12-19  
**Author**: AI Agent (Gemini)  
**Status**: Implemented (Needs Improvement)  
**Priority**: High improvement priority
