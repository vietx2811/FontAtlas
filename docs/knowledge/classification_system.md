# FontAtlas Classification System

## Overview
FontAtlas uses a multi-factor classification system to categorize fonts into:
- **Sans** (Sans-serif)
- **Serif**
- **Script** (Handwriting/Display script)
- **Monospace**
- **Symbol**

## Algorithm Summary (v2.1)

### Detection Priority
1. **Symbol** → PANOSE bFamilyType = 5
2. **Monospace** → width_variance < 0.05
3. **Serif Detection** → Analyze 'I'/'l' outline complexity
4. **Script vs Serif** → Command count + curvature thresholds
5. **Fallback** → Stroke contrast

### Key Metrics

| Metric | Source | Usage |
|--------|--------|-------|
| `command_count` | 'I'/'l' outline | Sans vs Serif/Script |
| `curvature_ratio` | Anatomy analysis | Serif vs Script |
| `width_variance` | Glyph widths | Monospace detection |
| `stroke_contrast` | 'O' analysis | Fallback serif detection |

### Thresholds (v2.1)

```python
# Sans detection
command_count <= 6  →  sans

# Serif detection  
command_count >= 10 AND curvature < 0.90  →  serif

# Script detection
curvature > 0.95  →  script
command_count > 18 AND curvature > 0.80  →  script
```

## Test Results
- **Google Fonts**: 93.9% accuracy (31/33)
- **Local UTM Fonts**: 100% on tested samples

## Files
- `src/parsers/font_parser.py`: `detect_serifs()` method
- `UX/python-sidecar/font_classifier.py`: `_determine_primary_class()`

## Changelog
See [classification_changelog.md](./algorithms/classification_changelog.md)
