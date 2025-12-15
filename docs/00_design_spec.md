# FontAtlas

## Anatomy‑based Font Pairing System

---

## 0. Positioning Statement

FontAtlas is a professional typography analysis and font‑pairing system built on **type anatomy and geometry**, not trends or subjective style labels.\
The system treats each font as a **multidimensional visual object**, embeds it into a vector space, and allows designers to explore compatibility through explainable, anatomy‑level relationships.

FontAtlas does **not** decide what is “good” or “bad typography.”\
It exposes *why* fonts align, clash, or create tension — and leaves the creative decision to the designer.

---

## 1. Design Principles

1. **Anatomy First**\
   All decisions derive from measurable typographic structure: stroke, contrast, aperture, rhythm.

2. **Explainability Over Verdicts**\
   No hard scores. Every suggestion is accompanied by structural reasoning.

3. **Typography‑Native Interface**\
   The UI behaves like an editorial tool, not an AI playground.

4. **Contextual Intelligence**\
   Pairing behavior adapts by medium (Web / Branding / Book).

---

## 2. System Architecture Overview

### Pipeline

Font File (TTF/OTF) → Geometry Extraction → Perceptual Rendering → Vector Embedding → Vector Database (Qdrant) → Pairing & Explanation Engine → Typography UI

---

## 3. Font Vector Model (Full Specification)

### 3.1 Geometry Vector (Primary Layer)

Measured directly from font outlines and metrics.

- Serif Presence Score
- Stroke Contrast Ratio
- Stress Angle
- Curvature Distribution
- Aperture Openness
- Terminal Sharpness
- x‑Height Ratio
- Width Variance
- Counter Density

These values are normalized and form the **core structural signature** of a font.

---

### 3.2 Perceptual Vector (Supplementary Layer)

Glyphs rendered as neutral black‑on‑white images using a fixed glyph set:

`H O n a e g`

Rendered samples are embedded using OpenCLIP.

This layer captures:

- Soft vs rigid impression
- Visual texture density
- Humanist vs mechanical feel

Perceptual vectors are weighted lower than geometry.

---

### 3.3 Functional Statistics Layer

- Small‑size legibility estimate
- Screen rendering stability
- Print contrast behavior
- Language support flags (Vietnamese: Yes/No)

---

### 3.4 Final Vector Composition

```
FinalFontVector = normalize(
  GeometryVector * 0.55 +
  PerceptualVector * 0.30 +
  FunctionalStats * 0.15
)
```

Stored in Qdrant with metadata filters.

---

## 4. Pairing Logic (Conceptual, Explainable)

Pairing is evaluated across **axes**, not labels.

### Key Relationships

- Alignment: Similar values on critical axes
- Contrast: Intentional divergence on secondary axes
- Distance Thresholds: Prevents arbitrary pairing

### Output Categories

- Safe Pairing
- Controlled Contrast
- Experimental Tension

No absolute ranking is provided.

---

## 4.1 Pair Explanation Engine — Prompt Logic v1 (Core Intelligence)

This system relies on a **non‑verdict, explanation‑only prompt architecture**. Prompts are treated as deterministic analytical functions, not conversational agents.

The goal is not to *recommend*, but to **articulate structural reasoning** in designer‑legible language.

---

### 4.1.1 Prompt Design Principles

1. **Axis‑Bound Reasoning**  
   The model may only reason using explicitly provided vector axes.

2. **No Adjectives as Conclusions**  
   Words like *modern, elegant, playful* are forbidden as final judgments.

3. **Difference Over Score**  
   Distance is described comparatively, never numerically ranked.

4. **Assumption Visibility**  
   The model must acknowledge which layers (Geometry / Perceptual / Functional) are active.

---

### 4.1.2 Canonical Prompt — Pair Explanation

**System Role:**

> You are a typography analysis engine.  
> You do not evaluate quality.  
> You explain structural relationships between typefaces.

**Input:**

- Font A vector (axes + values)
- Font B vector (axes + values)
- Active layers and weights
- Context (Web / Branding / Book)

**Instruction:**

> Explain how Font A and Font B relate structurally.  
> Describe alignment and divergence across axes.  
> Do not recommend. Do not score.  
> Avoid stylistic adjectives as conclusions.

**Expected Output Structure:**

1. Shared structural traits (2–3 axes)
2. Divergent traits and their typographic effect
3. Resulting tension or harmony under the given context

---

### 4.1.3 Contrast‑Focused Prompt

Used to test *intentional tension*.

**Instruction:**

> Identify which secondary axes diverge most strongly between Font A and Font B.  
> Explain how this divergence could create hierarchy or emphasis.  
> Do not suggest use cases.

---

### 4.1.4 Layer Sensitivity Prompt

Used to validate epistemological robustness.

**Instruction:**

> Re‑evaluate the relationship between Font A and Font B with the Perceptual layer disabled.  
> Explain what changes in the analysis and what remains stable.

This prompt is used internally to detect over‑reliance on AI embeddings.

---

### 4.1.5 Failure‑Mode Prompt (Bias Detection)

**Instruction:**

> If the Perceptual layer weight exceeds the Geometry layer, explain why this weakens typographic certainty.  
> Identify which conclusions become unreliable.

This prevents the system from drifting into style‑label dominance.

---

### 4.1.6 Output Constraints

- No numerical scores
- No rankings
- No prescriptive language (e.g. *should be used for*)

All outputs must remain **descriptive, comparative, and structural**.

---

### 4.1.7 Why Prompts Precede UI

These prompts define:

- Tooltip language
- Explanation panels
- Cluster annotations

UI elements are **rendered views of this reasoning**, not independent logic.

---

Pairing is evaluated across **axes**, not labels.

### Key Relationships

- Alignment: Similar values on critical axes
- Contrast: Intentional divergence on secondary axes
- Distance Thresholds: Prevents arbitrary pairing

### Output Categories

- Safe Pairing
- Controlled Contrast
- Experimental Tension

No absolute ranking is provided.

---

## 5. UX & Interaction Design Specification

### 5.1 Visual Language

- Neutral grayscale UI
- Typography‑led hierarchy
- No gradients, no AI mascots
- Swiss / editorial spacing discipline

---

### 5.2 Screen 01 — Font Intake

**Purpose:** Introduce font as an object of study.

Elements:

- Drag‑and‑drop font upload
- Glyph preview grid
- Processing state (non‑animated, minimal)

Outcome: Font anatomy profile generated.

---

### 5.3 Screen 02 — Font Anatomy Profile

**Layout:**

Left column:

- Large glyph preview
- Adjustable size slider

Right column:

- Anatomy radar / axis bars
- Hover reveals glyph regions

Language support clearly indicated.

---

### 5.4 Screen 03 — Pairing Explorer

User selects:

- Base font
- Context (Web / Branding / Book)

System displays three sections:

1. Structural Alignment
2. Structural Contrast
3. Experimental Range

Each suggestion includes a structural explanation.

---

### 5.5 Screen 04 — Pair Explanation Panel

Displays:

- Axis comparison diagram
- Highlighted differences
- Structural notes (no adjectives like “modern”)

Example: "Both fonts share similar stroke modulation. Serif presence diverges intentionally, enabling hierarchy."

---

### 5.6 Screen 05 — Visual Composition Test

- Headline / Body toggle
- Background light/dark
- Paragraph density preview

No export pressure — exploration first.

---

## 5.7 Screen 06 — Library View (Global Analysis)

**Purpose:** Provide a macro‑level understanding of the user’s entire font library based on anatomy, not naming or classification.

This view only exists once a library (e.g. RightFont) has been scanned.

---

### 5.7.1 Anatomy Heatmap (Library‑Wide)

**Description:** A matrix‑style heatmap representing the distribution of anatomical properties across all fonts in the library.

**Rows:** Individual fonts (sortable, filterable)\
**Columns:** Anatomical properties

Example columns:

- Serif Presence (Geometry)
- Stroke Contrast (Geometry)
- Aperture Openness (Geometry)
- Curvature Mean (Geometry)
- x‑Height Ratio (Geometry)
- Perceptual Softness (Perceptual)
- Screen Stability (Functional)

**Visual Encoding:**

- Monochrome scale (light → dark)
- No red/green success signals

**Interaction:**

- Hover a cell → highlight corresponding glyph regions
- Click a column → sort library by that property

**Key Principle:** Every column header clearly displays:

- Property name
- **Source of truth** (Geometry / Perceptual / Functional / Metadata)

---

### 5.7.2 Property Source & Toggle System

Each property includes:

- Source badge:
  - G = Measured Geometry
  - P = Perceptual (AI‑assisted)
  - F = Functional
  - M = Metadata (descriptive only)

Users may:

- Toggle properties **on/off** for all downstream analysis
- Adjust property weighting (advanced mode)

Disabled properties:

- Are visually muted
- Are excluded from vector distance calculations

This makes the analytical assumptions **explicit and designer‑controlled**.

---

### 5.7.3 Cluster View — Vector Space Projection

**Description:** A 2D projection of the font vector space representing anatomical similarity.

Projection methods:

- UMAP (default)
- PCA (optional, explanatory)

**Each node:**

- One font
- Size = usage frequency (optional)
- Color = selected property gradient

---

### 5.7.4 Cluster View Controls

Users can:

- Choose which properties are active in the projection
- Toggle entire layers:
  - Geometry only
  - Geometry + Perceptual
  - Geometry + Functional

When a property is toggled:

- The projection recomputes in real time
- Visual clustering visibly shifts

This allows designers to **see how assumptions reshape the map**.

---

### 5.7.5 Cluster Interaction

- Hover a node → glyph preview
- Click a node → open Font Anatomy Profile
- Lasso select → compare multiple fonts

Optional overlays:

- Family boundaries
- Foundry (metadata, dashed outline)

---

## 5.8 Related Open‑Source Research & Prior Art (OSS Landscape)

This project does not emerge in a vacuum. FontAtlas deliberately builds upon — and departs from — several existing open‑source and academic efforts. Understanding these precedents clarifies **what FontAtlas inherits, what it rejects, and what it extends**.

---

### 5.8.1 Font Geometry & Parsing Foundations

**FontTools (Python, OSS)**\
The de‑facto standard for parsing OpenType / TrueType fonts.

Used capabilities:

- Glyph outline extraction
- Metrics (x‑height, cap height, widths)
- Unicode coverage

**Limitation:** FontTools exposes *data*, not *interpretation*. It does not model relationships between glyph anatomy or perceptual impact.

FontAtlas uses FontTools strictly as a **measurement substrate**.

---

### 5.8.2 Shape & Outline Analysis

**skia‑pathops / Shapely**\
Used in computational geometry projects to analyze Bézier paths.

Prior uses:

- Boolean operations on glyphs
- Stroke normalization experiments

**Gap in prior art:** These libraries are rarely used to derive **semantic anatomical metrics** (e.g. aperture openness, terminal behavior).

FontAtlas explicitly elevates outline math into **typographic meaning**.

---

### 5.8.3 Perceptual Font Embedding Research

**DeepFont (Wang et al., 2015)**\
CNN‑based font classification from raster images.

**CLIP‑based Font Embedding (various OSS experiments)**

- Glyphs embedded via vision models
- Often optimized for font *identification* or *style labeling*

**Critical limitation:** Most approaches:

- Collapse fonts into style adjectives
- Optimize for prediction accuracy, not explainability

FontAtlas diverges by:

- Using perceptual vectors as *secondary signals*
- Never treating AI embeddings as authoritative truth

---

### 5.8.4 Font Pairing Algorithms (Academic)

**O’Donovan et al. — Exploratory Font Selection (2014)**

- Uses crowdsourced attributes
- Relies on human‑labeled descriptors

**Jiang et al. — Visual Font Pairing (2019)**

- ML‑based similarity scoring
- Outputs ranked pairings

**Limitation across this class:**

- Pairing framed as optimization problem
- Designers positioned as consumers of rankings

FontAtlas rejects ranking‑first logic in favor of **structural transparency**.

---

### 5.8.5 Network & Embedding Analogues

**UMAP / PCA (OSS)**\
Used widely in:

- Bioinformatics
- Material science
- Linguistics

**Analogy:** Fonts are treated like molecular structures — proximity implies similarity, not quality.

FontAtlas adopts this epistemology directly.

---

### 5.8.6 Vector Databases (Infrastructure OSS)

**Qdrant / FAISS / Milvus**

Common usage:

- Semantic search
- Recommendation engines

FontAtlas re‑purposes vector DBs as:

- **Analytical substrates**, not recommender systems
- Distance = structural divergence, not preference

---

### 5.8.7 What Is Missing in Existing OSS (Design Gap)

Across OSS and academic work, no system simultaneously provides:

- Anatomy‑grounded metrics
- Perceptual signals with controllable influence
- Explicit assumption toggling
- Typography‑native UI

FontAtlas is intentionally positioned to occupy this gap.

---

### 5.8.8 Summary Positioning

FontAtlas is best understood as:

> *An anatomy‑first visual analytics system for typography, borrowing infrastructure from ML, but epistemology from design.*

It stands adjacent to — not inside — existing font classification and pairing tools.

---

## 6. Technical Stack (Open & Free)

- Backend: Python
- Font Parsing: FontTools
- Geometry Analysis: shapely / skia-pathops
- Perceptual Embedding: OpenCLIP
- Vector DB: Qdrant (Docker)
- Frontend: React / Next.js
- Rendering: Canvas / SVG

---

## 6.1 Library Ingestion & Knowledge Enrichment

FontAtlas is designed to **augment existing font libraries**, not replace them.

### RightFont Library Integration

The system should allow users to:

- Scan local **RightFont libraries** (or similar font managers)
- Extract:
  - Font files (TTF / OTF)
  - Family & style relationships
  - Activation status
- Automatically ingest fonts into the FontAtlas pipeline

This enables designers to analyze **their real working library**, not a curated demo set.

RightFont is treated as a **trusted local source**, with no cloud dependency required.

---

### Web-based Metadata Crawling (Optional, Assisted)

To enrich the font database beyond geometry, FontAtlas may optionally crawl public information sources to attach **contextual metadata**.

Potential sources include:

- Foundry websites
- Public font documentation pages
- Open font repositories
- Academic / typographic references

Crawled data may include:

- Designer / foundry name
- Release year
- Intended usage notes (as stated by the foundry)
- Historical classification (humanist, grotesk, transitional, etc.)

All crawled information is:

- Stored as **non-authoritative metadata**
- Clearly separated from anatomy-derived data
- Never used directly for pairing decisions

---

### Knowledge Layer Separation Principle

FontAtlas strictly separates:

- **Measured anatomy** (geometry vectors)
- **Perceptual embeddings** (visual AI)
- **Descriptive metadata** (crawled or declared)

This prevents cultural or marketing language from polluting structural analysis.

---

## 7. Product Philosophy

FontAtlas is not a shortcut to good typography.

It is a **diagnostic instrument** — revealing structure, compatibility, and tension.

The system respects typography as a craft, not a dataset.

---

## 8. Future Extensions (Optional)

- Multi‑font system analysis (3+ fonts)
- Project‑level typography coherence score
- Designer‑tuned axis weighting
- Local‑only mode for sensitive studios

---

## Closing Note

Typography is geometry experienced emotionally.

FontAtlas exists to make that geometry visible — without flattening it into rules.

