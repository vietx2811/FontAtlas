# Curvature Distribution

## Context

Curvature distribution đo tỷ lệ curved segments vs straight segments trong font. Đây là metric để phân biệt:
- **Geometric fonts**: Nhiều đường thẳng, ít curves (Futura, Avant Garde)
- **Humanist fonts**: Nhiều curves, ít đường thẳng (Optima, Gill Sans)
- **Construction method**: Compass-and-ruler vs hand-drawn

## Research

**Typography theory:**
- Geometric fonts: Built from circles, rectangles → nhiều straight segments
- Humanist fonts: Inspired by calligraphy → nhiều organic curves
- This affects the "feel" of the font

**Measurement approach:**
- Count outline commands in glyph data
- `lineTo` = straight segment
- `curveTo` / `qCurveTo` = curved segment
- Ratio indicates construction style

## Decision

Đơn giản: **Count curves vs lines** trong lowercase alphabet.

**Formula:**
```python
curve_ratio = total_curves / (total_lines + total_curves)
```

**Interpretation:**
- 0.0 = All straight lines (pure geometric)
- 1.0 = All curves (pure organic)
- 0.3-0.5 = Geometric leaning
- 0.6-0.8 = Humanist leaning

## Implementation Notes

**File**: [`src/parsers/font_parser.py:652`](file:///Users/vietx/App%20Code/FontAtlas/src/parsers/font_parser.py#L652-L697)

**Algorithm:**
1. Sample all lowercase letters (a-z)
2. For each glyph outline:
   - Count `lineTo` commands
   - Count `curveTo` + `qCurveTo` commands
3. Calculate ratio: `curves / (lines + curves)`

**Return value**: 0.0 (geometric/straight) to 1.0 (humanist/curved)

**Fallback**: 0.5 on errors

## Trade-offs

### Pros
- ✓ Very simple and fast
- ✓ Directly measures outline construction
- ✓ Samples entire alphabet (26 glyphs)
- ✓ Clear semantic meaning

### Cons
- ✗ All curves weighted equally (no curvature amount)
- ✗ Doesn't account for curve complexity
- ✗ A single straight line connecting many curves still counts as "line"

## Validation

**This metric is relatively solid** compared to others.

The curve-to-line ratio genuinely reflects font construction philosophy:
- Geometric fonts ARE built with straight segments
- Humanist fonts DO use more bezier curves

**Expected to correlate well** with actual geometric vs humanist classification.

## Future Improvements

### Priority: Low (Already reasonable)

**Potential refinements:**

1. **Weight by segment length:**
   ```python
   for cmd, points in outline:
       length = calculate_segment_length(cmd, points)
       if cmd == 'lineTo':
           total_line_length += length
       else:
           total_curve_length += length
   
   ratio = total_curve_length / (total_line_length + total_curve_length)
   ```

2. **Curvature amount:** Measure how "curved" each curve is
   - Slight curve → low curvature score
   - Sharp curve → high curvature score

3. **Weighting by character**: Some characters more diagnostic than others
   - 'o', 'a', 'g' are more revealing than 'i', 'l'

## Related Knowledge

- [Terminal Sharpness](terminal_sharpness.md) - Also analyzes outline structure
- [Classification Logic](../decisions/classification_logic.md)
- [Font Anatomy](../research/font_anatomy.md)

## Expected Values

**Reasonable estimates:**
- Futura: 0.2-0.3 (very geometric)
- Helvetica: 0.4-0.5 (moderate)
- Gill Sans: 0.6-0.7 (humanist)
- Bodoni: 0.7-0.8 (highly curved)

**Worth validating** with actual test fonts.

---

**Created**: 2025-12-19  
**Last Updated**: 2025-12-19  
**Author**: AI Agent (Gemini)  
**Status**: Implemented (Good baseline)  
**Priority**: Low improvement priority
