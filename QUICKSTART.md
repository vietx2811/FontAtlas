# FontAtlas - Quick Start Guide

**Interactive CLI Ready!** 🎉

---

## Launch Interactive Menu

```bash
cd /Users/vietx/App\ Code/FontAtlas
source venv/bin/activate

python src/fontatlas.py
```

---

## Main Menu Options

```
╔════════════════════════════════════════════╗
║         FontAtlas - Font Pairing           ║
║           Anatomy-Based Analysis           ║
╚════════════════════════════════════════════╝

1. 📚 Browse Font Library
2. 🔍 Find Complementary Pairs  
3. 📊 View Font Profile
4. ⚙️  Batch Analyze New Fonts (coming soon)
5. 💾 Export Pairing Report (coming soon)
6. ℹ️  System Status
7. ❌ Exit
```

---

## Features Demonstrated

### ✅ Font Browser
Beautiful table showing:
- Font names
- Categories (Serif/Sans Serif)
- X-height ratios
- Width variance
- Glyph counts
- Vietnamese support

### ✅ Interactive Pair Finder
- Arrow key font selection
- Adjustable result count
- Compact or detailed explanations
- Colored score indicators

### ✅ Profile Viewer
- Formatted display panels
- All 4 data layers visible
- Metrics, coverage, metadata, perceptual

### ✅ System Status
- Qdrant connection health
- Indexed font count
- Collection info

---

## Legacy CLI Commands

Still available for scripting:

```bash
# Direct pair finding
python src/pairing/finder.py fonts/samples/roboto.complete.json --limit 3

# With detailed explanations
python src/pairing/finder.py fonts/samples/roboto.complete.json --limit 2 --explain

# Browse fonts (table only)
python src/cli/font_browser.py

# View profile
python src/cli/profile_viewer.py
```

---

## What's Complete

**Phase 1:**
- ✅ Font parsing (FontTools)
- ✅ Metadata crawling (Google Fonts)
- ✅ Perceptual embeddings (OpenCLIP)
- ✅ Complete profiles (4 layers)

**Phase 2:**
- ✅ Qdrant vector database
- ✅ Multi-factor pairing scorer
- ✅ Complementary finder
- ✅ Anatomy-based explainer
- ✅ Interactive CLI menu

---

## Next: Phase 3

Custom anatomy metrics:
- Stroke contrast analysis
- Aperture detection
- Terminal classification
- Stress angle
- Re-weighted vector composition

---

**Current System:** Production-ready backend + beautiful CLI
**Status:** Phase 2 Complete ✅
