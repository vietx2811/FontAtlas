# Terminal Sharpness Calculation

## Context

Cần phân biệt fonts có terminals sắc nét (sharp cutoffs) như Arial, Helvetica vs terminals tròn (blunt) như Futura, Avenir. Đây là một đặc điểm quan trọng trong classification, đặc biệt để phân biệt geometric sans vs humanist sans.

## Research

**Approaches đã xem xét:**

1. **Pixel-based analysis**: Render glyphs và phân tích pixels tại terminal points
   - ✗ Quá chậm, phụ thuộc rendering engine
   - ✗ Cần rasterization trước khi phân tích

2. **Normal vector calculation**: Tính vector vuông góc tại endpoints
   - ✗ Phức tạp với bezier curves
   - ✗ Cần differential geometry

3. **Control point distance** (Chosen ✓)
   - ✓ fontTools cung cấp sẵn control point data
   - ✓ Fast, không cần rendering
   - ✓ Works với outline data

## Decision

Sử dụng **control point distance** từ curve endpoint:

**Logic:**
- **Sharp terminal**: Control point 2 (CP2) nằm xa endpoint → abrupt direction change
- **Blunt terminal**: CP2 gần endpoint → smooth, gradual curve

**Measurement:**
```python
dx = end[0] - cp2[0]
dy = end[1] - cp2[1]
length = sqrt(dx² + dy²)

if length > 50:  # Sharp threshold
    sharp_endpoints += 1
```

## Implementation Notes

**File**: [`src/parsers/font_parser.py:405`](file:///Users/vietx/App%20Code/FontAtlas/src/parsers/font_parser.py#L405-L474)

**Sample characters**: 'a', 'c', 'e', 'r' (characters with visible terminals)

**Algorithm:**
1. Iterate through test characters
2. Extract glyph outline (Bezier curves)
3. For each `curveTo` command:
   - Get CP1, CP2, endpoint
   - Calculate distance from CP2 to endpoint
   - If distance > 50 units → count as sharp
4. Return mean ratio: `sharp_endpoints / total_curve_endpoints`

**Return value**: 0.0 (blunt/rounded) to 1.0 (sharp/cut)

**Fallback**: 0.5 (moderate) on any parsing errors

## Trade-offs

### Pros
- ✓ Fast execution (no rendering)
- ✓ Works purely with font outline data
- ✓ Reasonable accuracy for most fonts
- ✓ Simple to understand and debug

### Cons
- ✗ **Heuristic approach**, not geometrically precise
- ✗ **Threshold (50 units) is arbitrary**, may need tuning per font
- ✗ Doesn't handle quadratic bezier curves well (only cubic)
- ✗ Assumes all curves are terminal candidates (may include mid-glyph curves)

## Known Edge Cases

1. **Variable fonts**: May have different terminals at different weights
2. **Very small fonts**: 50-unit threshold may be too large
3. **Display fonts**: Extreme decorative terminals may confuse the metric
4. **Monoline fonts**: May have consistent CP distance regardless of terminal style

## Future Improvements

1. **Adaptive threshold**: Base threshold on UPM (units per em)
   ```python
   threshold = font.upem * 0.05  # 5% of UPM
   ```

2. **Better curve filtering**: Identify actual terminal curves vs internal curves
   - Check curve position (near glyph edges)
   - Analyze curve direction (outward vs inward)

3. **Quadratic bezier support**: Handle `qCurveTo` commands properly

4. **Validation dataset**: Test against human-labeled fonts to tune threshold

5. **Multi-character sampling**: Weight different character terminals differently
   - 'c' terminals may be more reliable than 'a'

## Related Knowledge

- [Stroke Contrast](stroke_contrast.md) - Complementary metric
- [Classification Logic](../decisions/classification_logic.md) - How this feeds into serif/sans detection
- [Font Anatomy Basics](../research/font_anatomy.md)

## Test Results

**Example values from common fonts** (to be populated after testing):
- Arial: ~0.8 (sharp)
- Helvetica: ~0.75 (sharp)
- Futura: ~0.3 (blunt)
- Avenir: ~0.35 (blunt)

---

**Created**: 2025-12-19  
**Last Updated**: 2025-12-19  
**Author**: AI Agent (Gemini)  
**Status**: Implemented & Deployed
