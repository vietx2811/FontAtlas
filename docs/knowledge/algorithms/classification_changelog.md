# Font Classification Algorithm - Changelog

## Version History

### v4.1.0 - 2025-12-22: Deepened Keyword Detection
**Enhanced FUNCTION classification with extended keyword lists**

#### Improvements
- Extended `DISPLAY_KEYWORDS`: Added 'press start', 'bangers', 'bebas', 'neue', 'oswald', 'condensed', 'narrow', 'fatface', 'abril'.
- Added specific checks for weight-based display detection (e.g., 'black', 'heavy', 'fat').
- Improved detection of modern condensed/poster fonts.

#### Files Modified
- `UX/python-sidecar/font_classifier.py`: Updated `DISPLAY_KEYWORDS` and detection logic.

---

### v4.0.0 - 2025-12-22: Two-Dimensional Classification
**Added STRUCTURE + FUNCTION taxonomy**

#### Classification Model
Based on user's design:

| Dimension | Values |
|-----------|--------|
| **STRUCTURE** | serif, sans, script, blackletter, monospace, symbol |
| **FUNCTION** | text, display, decorative |

#### FUNCTION Matrix
```
           Text    Display    Decorative
Serif       ✓        ✓           ✓
Sans        ✓        ✓           ✓
Script      ✕        ✓           ✓
Blackletter ✕        ✓           ✓
```

#### Detection Methods
- STRUCTURE: Glyph analysis (outline complexity, symmetry, curvature)
- FUNCTION: Font name keywords + metrics (x-height, weight, contrast)

#### Test Results
- STRUCTURE: 75/75 (100%)
- FUNCTION: 14/14 (100%) on test set

---

### v3.4.0 - 2025-12-22: Italic Font Handling


**Using symmetry to distinguish serif from script**

User insight: Serif/Sans have symmetric H, A, I. Script fonts have asymmetric forms.

```python
if has_serifs and symmetry_score < 0.85:
    return 'script'  # Asymmetric = script
```

---

### v3.1.0 - 2025-12-22: Symmetry Detection


**Achieved 100% accuracy on 38 Google Fonts**

#### Key Decision
- **Display is a style/tag, not a primary class**
- Fonts are classified by anatomy: serif, sans, script, monospace, symbol, blackletter
- "Display fonts" like Lobster are classified as serif (complex outlines)

---

### v2.2.0 - 2025-12-22: Stroke Contrast Integration





#### Problem Solved
- Previous algorithm based on `curvature` metric was too sensitive
- Fonts like "UTM American Sans" and "UTM Alter Gothic" were misclassified as `serif`

#### New Approach: Command Count Analysis
Analyze outline complexity of 'I' and 'l' glyphs:

| Classification | Command Count | Description |
|----------------|---------------|-------------|
| **Sans** | 5-6 | Simple rectangle stroke |
| **Serif** | 10-12 | Rectangle + serif decorations |
| **Script** | 15+ | Many curves and complex paths |

#### Logic Flow
```
1. Symbol → PANOSE bFamilyType=5
2. Monospace → width_variance < 0.05
3. Serif Detection (NEW):
   - Count outline commands for 'I' and 'l'
   - ≤6 commands → sans
   - ≥10 commands → check if serif or script
   - Script: cmd_count > 15 AND curvature > 0.55
4. Fallback → stroke contrast for edge cases
```

#### Test Results
| Font | Old Class | New Class | Status |
|------|-----------|-----------|--------|
| UTM American Sans | serif | sans | ✅ FIXED |
| UTM Alter Gothic | serif | sans | ✅ FIXED |
| UTM A&S Graceland | script | script | ✅ OK |
| Beautique Display | serif | serif | ✅ OK |

#### Files Modified
- `src/parsers/font_parser.py`: Added `detect_serifs()` using command count
- `UX/python-sidecar/font_classifier.py`: Updated `_determine_primary_class()`

---

### v1.0.0 - Initial Implementation
**Curvature-based classification**

#### Approach
- Use `curvature_ratio` from anatomy metrics
- Script: curvature > 0.60
- Serif: stroke_contrast >= 2.0 AND curvature > 0.40
- Sans: default

#### Issues Identified
- False positives for script (curvature threshold too low)
- Many sans fonts have high curvature but are misclassified
