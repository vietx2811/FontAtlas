# FontAtlas Documentation

Tài liệu thiết kế và research cho FontAtlas - Hệ thống phân tích và ghép cặp font dựa trên giải phẫu typography.

## 📁 Cấu trúc

### `00_design_spec.md`
**Design Specification & Framework**
- Positioning statement và design principles
- System architecture overview
- Font vector model (Geometry + Perceptual + Functional)
- Pairing logic và explanation engine
- UX/Interaction design specification (6 screens)
- Related OSS research & prior art
- Technical stack và product philosophy

### `01_tools_research.md`
**OSS Tools & Implementation Research**
- Phân tích foundation tools (FontTools, Shapely)
- State-of-the-art ML approaches (FontCLIP, Monotype AI)
- Anatomy metrics implementation gap analysis
- Detailed algorithms cho custom metrics:
  - Stroke Contrast Ratio
  - Aperture Openness
  - Terminal Sharpness
  - Stress Angle
  - Curvature Distribution
- 12-week implementation roadmap
- Technical decisions & resources

## 🎯 Quick Reference

**Core Philosophy:**
> Typography is geometry experienced emotionally.

**System Pipeline:**
```
Font File → Geometry Extraction → Perceptual Rendering → 
Vector Embedding → Vector DB (Qdrant) → Pairing Engine → UI
```

**Vector Composition:**
```
Final = Geometry (55%) + Perceptual (30%) + Functional (15%)
```

## 🔬 Key Findings

- ❌ **Không có OSS tool sẵn** cho anatomy metrics chuyên sâu
- ✅ **FontTools + Shapely** = Nền tảng vững để build custom
- 🎯 **FontAtlas differentiator**: Explainable structure, không phải rankings

## 📚 References

- Academic papers: FontCLIP (2024), Visual Font Pairing (2018)
- Industry tools: Monotype AI, FontJoy, Google Fonts
- Typography knowledge: Bringhurst, UXcel, Monotype glossary
