# Aperture Openness

## Context

Aperture là khoảng mở của characters như 'c', 'e', 's'. Aperture openness ảnh hưởng:
- **Legibility**: Wide apertures dễ đọc hơn ở size nhỏ
- **Style classification**: 
  - Display fonts: Narrow apertures
  - Text fonts: Wide apertures
  - Humanist: Generally wider
  - Geometric: Can be narrow

## Research

**Typography principle:**
- Aperture width = distance between terminals of 'c'
- Measured relative to character width
- Smaller apertures = more closed characters

**Measurement challenges:**
- How to identify "opening" in outline data?
- Different on top/bottom/side depending on character
- 'e' has horizontal aperture, 'c' has side aperture

**Chosen approach:**
- Use character **width relative to x-height** as proxy
- Wider characters (relative to height) tend to have more open apertures

## Decision

Sử dụng **relative width** của characters 'c', 'e', 's':

**Formula:**
```python
relative_width = char_width / x_height
aperture = (relative_width - 0.4) / 0.6
aperture = clamp(aperture, 0.0, 1.0)
```

**Mapping:**
- Narrow (0.4-0.6 of x-height) → 0.0-0.33 (closed)
- Medium (0.6-0.8) → 0.33-0.67 (moderate)
- Wide (0.8-1.0) → 0.67-1.0 (open)

## Implementation Notes

**File**: [`src/parsers/font_parser.py:594`](file:///Users/vietx/App%20Code/FontAtlas/src/parsers/font_parser.py#L594-L650)

**Algorithm:**
1. Get glyphs for 'c', 'e', 's'
2. Calculate bbox for each
3. Get x-height from OS/2 table (or bbox height as fallback)
4. Calculate `width / x_height` ratio
5. Map ratio to 0.0-1.0 scale
6. Return mean across all sampled characters

**Return value**: 0.0 (closed/narrow) to 1.0 (wide open)

**Fallback**: 0.5 on errors

## Trade-offs

### Pros
- ✓ Fast (bbox calculation only)
- ✓ Samples multiple characters
- ✓ Reasonable correlation with actual aperture

### Cons
- ✗ Character width ≠ aperture opening
- ✗ Doesn't measure actual gap
- ✗ Influenced by overall character proportions
- ✗ Magic numbers (0.4, 0.6) are arbitrary

## Known Issues

**Width ≠ Aperture:** A wide character doesn't necessarily have wide apertures.

Example: 
- 'e' could be wide overall but have closed counter opening
- Decorative fonts may have wide characters with narrow apertures

**Better approach needed** to actually measure the gap itself.

## Future Improvements

### Priority: Medium

**Measure actual opening:**

```python
def measure_aperture_gap(outline, char):
    """Measure the narrowest point of the opening"""
    if char == 'c':
        # Scan vertically at the opening (right side)
        opening_x = find_opening_position()
        gap = measure_vertical_spacing(outline, opening_x)
    
    elif char == 'e':
        # Scan horizontally where counter opens
        middle_y = find_counter_middle()
        gap = measure_horizontal_spacing(outline, middle_y)
    
    # Normalize by character size
    return gap / char_height
```

**Alternative:**
1. Find counter (internal white space)
2. Identify "opening" (where counter connects to exterior)
3. Measure width at that connection point

## Related Knowledge

- [Counter Density](counter_density.md) - Related counter measurement
- [Terminal Sharpness](terminal_sharpness.md) - Also analyzes character endings
- [Classification Logic](../decisions/classification_logic.md)

## Expected Values

**Hypothetical** (current implementation may differ):
- Helvetica: 0.6-0.7 (moderate-open)
- Futura: 0.4-0.5 (closed)
- Gill Sans: 0.7-0.8 (open)
- Display fonts: 0.2-0.4 (very closed)

---

**Created**: 2025-12-19  
**Last Updated**: 2025-12-19  
**Author**: AI Agent (Gemini)  
**Status**: Implemented (Approximation)  
**Priority**: Medium improvement priority
