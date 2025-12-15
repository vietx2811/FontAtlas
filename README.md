# FontAtlas

> **Anatomy-based Font Pairing System**  
> Professional typography analysis built on type anatomy and geometry, not trends or style labels.

## Philosophy

Typography is geometry experienced emotionally.

FontAtlas treats each font as a **multidimensional visual object**, embeds it into a vector space, and allows designers to explore compatibility through explainable, anatomy-level relationships.

**We don't decide what is "good" typography.** We expose *why* fonts align, clash, or create tension — and leave the creative decision to you.

---

## System Architecture

```
Font File (TTF/OTF)
    ↓
Geometry Extraction (FontTools + Custom)
    ↓
Perceptual Rendering (OpenCLIP)
    ↓
Vector Embedding (Geometry 55% + Perceptual 30% + Functional 15%)
    ↓
Vector Database (Qdrant)
    ↓
Pairing & Explanation Engine
    ↓
Typography UI (React/Next.js)
```

---

## Project Structure

```
FontAtlas/
├── docs/                    # Design specs & research
│   ├── 00_design_spec.md    # Full framework specification
│   ├── 01_tools_research.md # OSS tools & implementation research
│   └── README.md
│
├── src/                     # Python backend
│   ├── parsers/             # FontTools integration
│   ├── metrics/             # Anatomy metrics (basic + custom)
│   ├── embeddings/          # OpenCLIP perceptual vectors
│   ├── crawlers/            # Metadata from Google Fonts, etc.
│   ├── database/            # Qdrant vector DB
│   └── api/                 # FastAPI endpoints
│
├── tests/                   # Unit tests
└── fonts/                   # Sample fonts for testing
```

---

## Development Phases

### ✅ Phase 0: Documentation & Research
- Design specification complete
- OSS tools research done
- Implementation roadmap defined

### 🔄 Phase 1: OSS-Supported Features (Current)
- FontTools integration
- Basic metrics (x-height, width, coverage)
- Metadata crawling (Google Fonts API)
- OpenCLIP perceptual embeddings

### 📋 Phase 2: Vector Database & Simple Pairing
- Qdrant setup
- Initial vector composition
- Similarity search API

### 📋 Phase 3: Custom Anatomy Metrics
- Stroke contrast analysis
- Aperture detection
- Terminal classification
- Advanced geometry (stress angle, curvature)

### 📋 Phase 4: UI & Explanation Engine
- React/Next.js interface
- LLM-based structural explanations
- Library view (heatmap, UMAP clusters)

---

## Tech Stack

**Backend:**
- Python 3.11+
- FontTools - Font parsing
- Shapely - Geometric analysis
- OpenCLIP - Visual embeddings
- Qdrant - Vector database
- FastAPI - API layer

**Frontend (Future):**
- React / Next.js
- Canvas / SVG rendering
- D3.js / Recharts for visualizations

---

## Getting Started

### Prerequisites
```bash
# Python 3.11+
python --version

# Install dependencies
pip install fonttools shapely open-clip-torch qdrant-client fastapi
```

### Quick Start
```bash
# Clone repository
cd FontAtlas

# Install dependencies
pip install -r requirements.txt

# Run font parser (Phase 1A)
python src/parsers/font_parser.py --font fonts/Roboto-Regular.ttf
```

---

## Documentation

See [`docs/README.md`](docs/README.md) for full design specifications and research.

**Key Documents:**
- `00_design_spec.md` - Complete system design
- `01_tools_research.md` - OSS analysis & implementation strategies

---

## License

MIT (to be confirmed)

---

## Roadmap

Track progress in [`task.md`](https://github.com/user/.gemini/antigravity/brain/.../task.md)

**Current Focus:** Phase 1A - FontTools integration & basic metrics
