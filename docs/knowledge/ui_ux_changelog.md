# UI/UX Redesign - Changelog

## Version History

### v2.0.0 - 2025-12-22: Redundant-free Detail Panel & Simplified Cards
**Major layout restructure for better readability and feature grouping.**

#### Library Cards
- **Simplified Design**: Metrics (x-height, contrast, etc.) removed from the default card view to reduce visual noise.
- **Preview Centric**: Cards now focus on the font name and a large "Aa Bb Cc" preview.
- **Interactive**: Added a "View details" hint that appears on hover.

#### Detail Panel
- **Consolidated Header**: All identity information (Name, Family, Category, Classification, Quick Stats) moved to a dark-themed header to eliminate redundancy.
- **Tabbed Navigation**: Reorganized content into 5 logical tabs:
    1. **Preview**: Dedicated zone for type specimen and custom text testing.
    2. **Metrics**: Academic breakdown with formulas and interpretation hints.
    3. **Info**: Metadata and external links (Google Fonts, MyFonts).
    4. **Pairing**: AI-powered font pairing engine.
    5. **Similar**: Integrated visual similarity search (moved from card icon).
- **Academic Metrics**: Added calculation formulas (e.g., `height_x / height_cap`) and range indicators to the metrics tab for a more professional feel.
- **Expandable Sections**: Used `<details>` blocks for optional information like custom preview inputs and metadata to keep the primary view clean.

#### Files Modified
- `UX/components/FontCard.tsx`: Simplified card layout.
- `UX/components/DetailPanel.tsx`: Full redesign of the panel and tab system.
- `UX/components/VirtualizedFontGrid.tsx`: Updated according to new component props.
- `UX/App.tsx`: Integrated new panel navigation.

---

### v1.0.0 - Initial Detail Panel
- Basic font information display.
- Integrated `MetricsChart` (radar chart).
- Added custom text preview slider.
