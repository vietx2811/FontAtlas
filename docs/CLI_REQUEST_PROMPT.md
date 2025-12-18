# FontAtlas TUI Version - Request Document

> **Purpose**: This document provides all necessary context and specifications for an AI to build a comprehensive **TUI (Text User Interface)** version of FontAtlas — a rich, interactive terminal **font manager application** inspired by tools like Midnight Commander, Ranger, and Superfile.

---

## 🎯 Project Overview

**FontAtlas** is an anatomy-based font pairing system — a professional font analysis and pairing tool for designers and typographers.

### Core Philosophy
> "Typography is geometry experienced emotionally."

FontAtlas treats each font as a **multidimensional visual object**, embeds it into a vector space, and allows designers to explore compatibility through explainable, anatomy-level relationships.

### Core Value Propositions
- **Analyze** font anatomy with scientific precision
- **Classify** fonts automatically (serif, sans, display, monospace, script, decorative)
- **Pair** fonts using AI-powered complementary matching
- **Compare** fonts side-by-side with metrics
- **Browse** large font collections efficiently

---

## 🏗️ System Architecture

```
Font File (TTF/OTF)
    ↓
Geometry Extraction (FontTools + Custom metrics)
    ↓
Perceptual Rendering (OpenCLIP embeddings)
    ↓
Vector Embedding (Geometry 55% + Perceptual 30% + Functional 15%)
    ↓
Vector Database (Qdrant)
    ↓
Pairing & Explanation Engine
    ↓
TUI Interface (Textual or Rich + Questionary)
```

---

## 🖥️ TUI Application Design

### Design Inspiration
Build this like a **font manager application** similar to:
- **Midnight Commander** (mc) - Split-pane file manager
- **Ranger** - Vim-like file manager with preview
- **Superfile** - Modern TUI file manager
- **lazygit** - Interactive git TUI
- **btop** - System monitor with panels

### Recommended Framework
Use **Textual** (Python TUI framework) for building a full-screen, reactive application:
```
pip install textual
```

Textual provides:
- Full-screen applications
- Reactive UI updates
- CSS-like styling
- Keyboard navigation
- Mouse support
- Multiple panes/panels

---

## 📐 Main Application Layout

### Full-Screen Three-Pane Design

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  FontAtlas                                    ~/fonts/google  ⬤ Qdrant Connected ┃
┣━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━┫
┃ DIRECTORIES           ┃ FONTS (150)              Filter: ┃ PREVIEW               ┃
┃───────────────────────┃──────────────────────────────────┃───────────────────────┃
┃ ★ ~/fonts/google      ┃                                  ┃                       ┃
┃   ~/fonts/custom      ┃  ▸ Roboto Regular    [sans]     ┃   Roboto Regular      ┃
┃   ~/fonts/adobe       ┃    Roboto Bold       [sans]     ┃   ━━━━━━━━━━━━━━━━    ┃
┃   /System/Fonts       ┃    Roboto Italic     [sans]     ┃                       ┃
┃                       ┃    Georgia           [serif]    ┃   ABCDEFGHIJKLM       ┃
┃───────────────────────┃    Montserrat        [sans]     ┃   NOPQRSTUVWXYZ       ┃
┃ CATEGORIES            ┃    Libre Baskerville [serif]    ┃   abcdefghijklm       ┃
┃───────────────────────┃    Merriweather      [serif]    ┃   nopqrstuvwxyz       ┃
┃ ● All (150)           ┃    Open Sans         [sans]     ┃   0123456789          ┃
┃ ○ Serif (45)          ┃    Playfair Display  [display]  ┃                       ┃
┃ ○ Sans-Serif (62)     ┃    Lato              [sans]     ┃   Thế giới xin chào   ┃
┃ ○ Display (28)        ┃    Source Code Pro   [mono]     ┃                       ┃
┃ ○ Monospace (10)      ┃    Fira Code         [mono]     ┃───────────────────────┃
┃ ○ Script (5)          ┃    Dancing Script    [script]   ┃ METRICS               ┃
┃                       ┃    ▾ (scroll for more...)       ┃ X-height:  0.722 ████ ┃
┃───────────────────────┃                                  ┃ Width:     0.150 █    ┃
┃ QUICK ACTIONS         ┃──────────────────────────────────┃ Stroke:    1.800 ██   ┃
┃───────────────────────┃ Sort: Name ▾  View: List         ┃ Aperture:  0.550 ███  ┃
┃ [P] Find Pairs        ┃                                  ┃───────────────────────┃
┃ [C] Compare           ┃                                  ┃ Vietnamese: ✓ Yes     ┃
┃ [S] Scan Directory    ┃                                  ┃ Category:  Sans-Serif ┃
┃ [I] Index to Qdrant   ┃                                  ┃ Glyphs:    892        ┃
┗━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━┛
┃ [↑↓] Navigate  [Enter] Select  [P] Pair  [C] Compare  [/] Search  [?] Help  [Q] Quit ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

### Panel Descriptions

#### Left Panel: Navigation
- **Directories**: List of font directories (starred = default)
- **Categories**: Filter by font classification
- **Quick Actions**: Keyboard shortcuts for common tasks

#### Center Panel: Font List
- Scrollable list of all fonts
- Shows font name + category badge
- Highlighted selection
- Filter/search bar at top
- Sort options at bottom

#### Right Panel: Live Preview
- **Font specimen**: Live rendered alphabet
- **Vietnamese sample**: "Thế giới xin chào"
- **Metrics visualization**: Bar charts for key metrics
- **Quick info**: Category, glyph count, features

---

## 🎯 View Modes

### 1. Browse Mode (Default)
The main three-pane layout shown above.

### 2. Pairing Mode
When user presses `[P]` or selects "Find Pairs":

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  FontAtlas > Pairing Mode                                     [ESC] Back to Browse ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
┃ QUERY FONT                                   ┃ COMPLEMENTARY PAIRS                  ┃
┃──────────────────────────────────────────────┃──────────────────────────────────────┃
┃                                              ┃                                      ┃
┃   ╔═══════════════════════════════════╗      ┃  1. Libre Baskerville    ⭐⭐⭐⭐⭐  ┃
┃   ║                                   ║      ┃     Score: 0.872                     ┃
┃   ║     Roboto Regular                ║      ┃     ┌────────────────────────────┐   ┃
┃   ║     ═══════════════               ║      ┃     │ Aa Bb Cc Dd 123            │   ┃
┃   ║                                   ║      ┃     │ Excellent contrast         │   ┃
┃   ║     Aa Bb Cc Dd Ee                ║      ┃     └────────────────────────────┘   ┃
┃   ║     Ff Gg Hh Ii Jj                ║      ┃                                      ┃
┃   ║     0123456789                    ║      ┃  2. Georgia              ⭐⭐⭐⭐    ┃
┃   ║                                   ║      ┃     Score: 0.845                     ┃
┃   ╚═══════════════════════════════════╝      ┃     ┌────────────────────────────┐   ┃
┃                                              ┃     │ Aa Bb Cc Dd 123            │   ┃
┃   Category: Sans-Serif                       ┃     │ Good width harmony         │   ┃
┃   X-height: 0.722 │ Width: 0.150             ┃     └────────────────────────────┘   ┃
┃   Stroke: 1.8     │ Aperture: 0.55           ┃                                      ┃
┃                                              ┃  3. Merriweather         ⭐⭐⭐⭐    ┃
┃──────────────────────────────────────────────┃     Score: 0.798                     ┃
┃ [↑↓] Select pair  [Enter] View details       ┃     ┌────────────────────────────┐   ┃
┃ [1-5] Quick select  [E] Explain all          ┃     │ Aa Bb Cc Dd 123            │   ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

### 3. Detail/Explanation Mode
When user selects a pair and presses Enter:

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  Pairing Analysis: Roboto + Libre Baskerville                 [ESC] Back          ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
┃                                                                                   ┃
┃   ┌─────────────────────┐    ┌─────────────────────┐                              ┃
┃   │   Roboto Regular    │    │ Libre Baskerville   │     OVERALL SCORE            ┃
┃   │   ───────────────   │    │ ─────────────────   │     ════════════════         ┃
┃   │   Aa Bb Cc Dd Ee    │ ⟷  │   Aa Bb Cc Dd Ee    │         0.872               ┃
┃   │   Sans-Serif        │    │   Serif             │      ⭐⭐⭐⭐⭐              ┃
┃   └─────────────────────┘    └─────────────────────┘                              ┃
┃                                                                                   ┃
┃   ╔═══════════════════════════════════════════════════════════════════════════╗   ┃
┃   ║ STRUCTURAL ANALYSIS                                                       ║   ┃
┃   ╠═══════════════════════════════════════════════════════════════════════════╣   ┃
┃   ║                                                                           ║   ┃
┃   ║  X-height Harmony    ████████████████████░░░░  0.92  ✓ Excellent          ║   ┃
┃   ║  ├─ Roboto: 0.722                                                         ║   ┃
┃   ║  └─ Libre:  0.710 (Δ 0.012)                                               ║   ┃
┃   ║                                                                           ║   ┃
┃   ║  Width Contrast      ████████████████████░░░░  0.88  ✓ Complementary      ║   ┃
┃   ║  └─ Creates visual rhythm without clashing                                ║   ┃
┃   ║                                                                           ║   ┃
┃   ║  Stroke Balance      ███████████████░░░░░░░░░  0.75  ○ Good               ║   ┃
┃   ║  ├─ Roboto: 1.8 (Low contrast)                                            ║   ┃
┃   ║  └─ Libre:  2.4 (Moderate) - Adds hierarchy                               ║   ┃
┃   ║                                                                           ║   ┃
┃   ║  Perceptual Match    ███████████████░░░░░░░░░  0.78  ✓ Strong harmony     ║   ┃
┃   ║                                                                           ║   ┃
┃   ║  Category Bonus      ─────────────────────────  1.2x  Serif + Sans        ║   ┃
┃   ║                                                                           ║   ┃
┃   ╚═══════════════════════════════════════════════════════════════════════════╝   ┃
┃                                                                                   ┃
┃   RECOMMENDATION: Use Libre Baskerville for headings, Roboto for body text.       ┃
┃                                                                                   ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
┃ [S] Save pair  [X] Export report  [Enter] Try live preview  [ESC] Back            ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

### 4. Compare Mode
Side-by-side font comparison:

```
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃  Compare Fonts (3 selected)                                    [ESC] Back         ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
┃  Roboto Regular          ┃  Georgia                 ┃  Montserrat               ┃
┃  ────────────────────────┃──────────────────────────┃───────────────────────────┃
┃                          ┃                          ┃                           ┃
┃  Aa Bb Cc Dd Ee          ┃  Aa Bb Cc Dd Ee          ┃  Aa Bb Cc Dd Ee           ┃
┃  Ff Gg Hh Ii Jj          ┃  Ff Gg Hh Ii Jj          ┃  Ff Gg Hh Ii Jj           ┃
┃  0123456789              ┃  0123456789              ┃  0123456789               ┃
┃                          ┃                          ┃                           ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━╋━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
┃  Category    [Sans]      ┃  Category    [Serif]     ┃  Category    [Sans]       ┃
┃  X-height    0.722       ┃  X-height    0.680 ↓     ┃  X-height    0.750 ↑      ┃
┃  Width var   0.150       ┃  Width var   0.180 ↑     ┃  Width var   0.120 ↓      ┃
┃  Stroke      1.800       ┃  Stroke      2.400 ↑     ┃  Stroke      1.500 ↓      ┃
┃  Aperture    0.550       ┃  Aperture    0.620 ↑     ┃  Aperture    0.480 ↓      ┃
┃  Vietnamese  ✓           ┃  Vietnamese  ✓           ┃  Vietnamese  ✓            ┃
┃  Glyphs      892         ┃  Glyphs      580         ┃  Glyphs      736          ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━┻━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
┃  PAIRING SCORES                                                                  ┃
┃  ├─ Roboto + Georgia:     0.872 ⭐⭐⭐⭐⭐ Excellent                              ┃
┃  ├─ Roboto + Montserrat:  0.623 ⭐⭐⭐    Fair (similar styles)                   ┃
┃  └─ Georgia + Montserrat: 0.845 ⭐⭐⭐⭐  Good                                    ┃
┣━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┫
┃ [A] Add font  [R] Remove font  [X] Export  [←→] Switch focus  [ESC] Back          ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

### 5. Scan/Index Modal
Overlay modal for scanning:

```
                    ┌──────────────────────────────────────────────────┐
                    │  Scanning Fonts...                               │
                    ├──────────────────────────────────────────────────┤
                    │                                                  │
                    │  Directory: ~/fonts/google                       │
                    │                                                  │
                    │  ████████████████████░░░░░░░░░░░░  62%           │
                    │                                                  │
                    │  Current: Playfair-Display-Bold.ttf              │
                    │  Step: 4/6 Analyzing stroke contrast...          │
                    │                                                  │
                    │  ┌─────────────────────────────────────────────┐ │
                    │  │ ✓ Roboto-Regular.ttf                        │ │
                    │  │ ✓ Roboto-Bold.ttf                           │ │
                    │  │ ✓ Georgia.ttf                               │ │
                    │  │ ⋯ Playfair-Display-Bold.ttf                  │ │
                    │  │   Montserrat-Regular.ttf                    │ │
                    │  │   Montserrat-Bold.ttf                       │ │
                    │  └─────────────────────────────────────────────┘ │
                    │                                                  │
                    │  Completed: 15 │ Errors: 0 │ ETA: ~2 min         │
                    │                                                  │
                    │  [ESC] Cancel                                    │
                    └──────────────────────────────────────────────────┘
```

---

## ⌨️ Keyboard Navigation

### Global Keys
| Key | Action |
|-----|--------|
| `Q` | Quit application |
| `?` | Show help overlay |
| `/` | Focus search/filter |
| `ESC` | Go back / Cancel |
| `Tab` | Switch panel focus |
| `1-9` | Quick select items |

### Browse Mode
| Key | Action |
|-----|--------|
| `↑↓` | Navigate font list |
| `←→` | Switch directory/category |
| `Enter` | View font profile |
| `P` | Find pairs for selected font |
| `C` | Add to comparison |
| `Space` | Toggle selection |
| `S` | Scan current directory |
| `I` | Index to Qdrant |

### Pairing Mode
| Key | Action |
|-----|--------|
| `↑↓` | Navigate pairs |
| `Enter` | View detailed analysis |
| `E` | Show all explanations |
| `S` | Save pair to favorites |
| `X` | Export report |

---

## 🛠️ Tech Stack

### Required Dependencies
```txt
# TUI Framework (choose one)
textual>=0.40.0              # Recommended: Full-featured TUI framework
# OR use rich for simpler approach:
# rich>=13.7.0
# questionary>=2.0.0

# Core font parsing
fonttools>=4.47.0
shapely>=2.0.2

# OpenCLIP for perceptual embeddings
open-clip-torch>=3.0.0
torch>=2.0.0
Pillow>=10.0.0

# Vector database
qdrant-client>=1.7.0

# Anatomy metrics
scipy>=1.11.0
scikit-image>=0.21.0
numpy>=1.24.0
```

### Application Structure
```
src/
├── tui/
│   ├── app.py              # Main Textual application
│   ├── screens/
│   │   ├── browse.py       # Browse mode screen
│   │   ├── pairing.py      # Pairing mode screen
│   │   ├── compare.py      # Compare mode screen
│   │   └── profile.py      # Font profile screen
│   ├── widgets/
│   │   ├── font_list.py    # Scrollable font list
│   │   ├── font_preview.py # Live font preview
│   │   ├── metrics_chart.py # Metrics visualization
│   │   ├── directory_tree.py # Directory browser
│   │   └── progress_modal.py # Scan progress
│   ├── styles/
│   │   └── theme.tcss      # Textual CSS styles
│   └── utils/
│       └── keybindings.py  # Key binding handlers
├── fontatlas.py            # Entry point
```

---

## 🎨 Styling (Textual CSS)

```css
/* theme.tcss */

Screen {
    background: #1a1a2e;
}

.panel {
    border: solid #3a3a5e;
    background: #16213e;
}

.panel-header {
    background: #0f3460;
    color: #e94560;
    text-style: bold;
}

.font-item {
    padding: 0 1;
}

.font-item:hover {
    background: #2a2a4e;
}

.font-item.selected {
    background: #0f3460;
    color: #ffffff;
}

.category-badge {
    background: #3a3a5e;
    color: #94a3b8;
    padding: 0 1;
}

.category-serif { color: #f97316; }
.category-sans { color: #22c55e; }
.category-display { color: #a855f7; }
.category-mono { color: #3b82f6; }
.category-script { color: #ec4899; }

.metrics-bar {
    color: #22c55e;
}

.score-excellent { color: #22c55e; }
.score-good { color: #eab308; }
.score-fair { color: #f97316; }
.score-poor { color: #ef4444; }

.status-bar {
    background: #0f3460;
    dock: bottom;
    height: 1;
}
```

---

## 📊 Data Structures

### Font Profile (`.complete.json`)
```json
{
  "metadata": {
    "font_name": "Roboto Regular",
    "file_path": "/path/to/Roboto-Regular.ttf",
    "file_format": "TrueType"
  },
  "basic_metrics": {
    "x_height_ratio": 0.722,
    "cap_height": 720,
    "width_variance": 0.15,
    "units_per_em": 1000
  },
  "anatomy_metrics": {
    "stroke_contrast": { "mean": 1.8, "classification": "Low" },
    "aperture_openness": { "mean": 0.55, "classification": "Moderate" }
  },
  "unicode_coverage": {
    "total_glyphs": 250,
    "supports_vietnamese": true
  },
  "metadata_layer": {
    "google_fonts": { "category": "sans-serif" },
    "classification": { "predicted_category": "sans-serif", "confidence": 0.92 }
  },
  "perceptual_layer": {
    "aggregated_embedding": [0.123, -0.456, ...]
  }
}
```

---

## ✅ Deliverables Checklist

- [ ] **Main Application** (`src/tui/app.py`)
  - Full-screen Textual application
  - Screen navigation
  - Global keybindings

- [ ] **Browse Screen** (`src/tui/screens/browse.py`)
  - Three-pane layout
  - Directory navigation
  - Font list with filtering/sorting
  - Live preview panel

- [ ] **Pairing Screen** (`src/tui/screens/pairing.py`)
  - Query font display
  - Pair results with scores
  - Detailed analysis view

- [ ] **Compare Screen** (`src/tui/screens/compare.py`)
  - Multi-font comparison (2-4)
  - Side-by-side metrics
  - Pairing scores matrix

- [ ] **Widgets**
  - Scrollable font list
  - Font preview with specimens
  - Metrics bar charts
  - Progress modal
  - Search/filter input

- [ ] **Styling**
  - Dark theme (default)
  - Color-coded categories
  - Score visualization

---

## 🚀 Launch Command

```bash
# Run the TUI application
python src/fontatlas.py

# Or with entry point
fontatlas
```

---

## 📝 Additional Requirements

1. **Persistent state**: Remember last directory, window sizes, favorites
2. **Fast startup**: Lazy load fonts, cache previews
3. **Responsive**: Handle terminal resize gracefully  
4. **Vietnamese focus**: Always show Vietnamese sample in preview
5. **Mouse support**: Clickable panels and items (Textual provides this)
6. **Error handling**: Graceful degradation if Qdrant unavailable

---

*This document specifies a professional font manager TUI application. Use Textual framework for best results. The interface should feel like a dedicated font management tool, not just a CLI with pretty output.*
