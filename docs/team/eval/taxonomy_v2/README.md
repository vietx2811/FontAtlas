# Bộ eval taxonomy v2 (classifier)

Soạn bởi ML (04), 2026-11-11. Taxonomy: `docs/team/research/2026-11-11-font-classification-taxonomy.md` mục 4 (8 class, 33 subclass).

## File

| File | Nội dung |
|---|---|
| `../taxonomy_v2_labels.csv` | 225 dòng / 224 họ OFL. Cột `source` (GF tag hoặc tay), `quality` (gf / visual / weak), `boundary`, `gf_tags`, `note` |
| `fonts.sha256` | SHA-256 của 958 file (`.ttf/.otf`, `METADATA.pb`, `OFL.txt`) của các họ trong bảng, tại `google/fonts@47d54ca1ebb14fd864695f1d39dc734f39017c33` |
| `fetch_fonts.sh <thư-mục-ngoài-repo>` | Sparse clone đúng commit trên, chỉ lấy các thư mục họ cần, rồi kiểm checksum |
| `baseline_old_classifier.md` | Kết quả classifier hiện tại (7 class cũ) quy về taxonomy v2 |

Font không được commit. Tune/test KHÔNG ghi trong CSV: dùng `fontatlas_embed::family_split(family)` như các eval khác.

## Chạy

```bash
docs/team/eval/taxonomy_v2/fetch_fonts.sh ~/fontatlas-ref/gf-taxonomy
cd UX/src-tauri && cargo run --release -p fontatlas-eval --bin classify_eval -- \
  --gf ~/fontatlas-ref/gf-taxonomy --taxonomy-v2 ../../docs/team/eval/taxonomy_v2_labels.csv [--split test|tune|all] [--dump out.csv] [--out out.md]
```

Mặc định in ba khối: `test`, `tune`, `all`. Số chính của classifier mới là khối `test`; chỉ chỉnh ngưỡng bằng `tune`.
Muốn đo classifier taxonomy v2 mới: thay thân hàm `predict_v2` trong `classify_eval.rs` (hiện ánh xạ classifier cũ bằng `taxonomy_v2::map_old_classifier`), phần còn lại giữ nguyên.

## Cách đọc nhãn

- **Cổng** = nhãn `quality` gf hoặc visual. Ma trận class và bảng subclass chỉ tính nhãn cổng; nhãn `weak` chỉ in một dòng tổng.
- **`gf`**: tag Google Fonts (điểm ≥ 50) khớp subclass. **`visual`**: gán tay sau khi xem glyph (contact sheet), lý do ở cột `note`. **`weak`**: gán tay hoặc tag nhiễu, ranh giới mờ hoặc taxonomy xếp là M (signature, marker, childlike, upright_script, coding, Bitter).
- **`secondary`** chỉ ghi để hiển thị (vd. flared sans có secondary `serif/glyphic`); eval chưa chấm.
- **`boundary`**: fixture ranh giới được in riêng, tính cả họ weak.
  - `tailed_l_sans` (Mooli, Bellota, Ruluko, Port Lligat Sans): chữ l có đuôi cong làm `serif_foot` ≈ 1,4–1,6 dù không có serif. Kỳ vọng Sans.
  - `small_serif_risk` (Marcellus, Marcellus SC): glyphic serif, `serif_foot` ≈ 1,3–1,4, sát ngưỡng sans. Kỳ vọng Serif.

## Ô thiếu họ (không dùng làm cổng) — `KNOWN_GAPS`

| Ô | Số họ OFL | Lý do |
|---|---|---|
| serif/wedge | 0 | GF không có tag; đã xem ~25 họ serif ứng viên, không họ nào xác nhận được wedge/Latin serif |
| mono/typewriter | 2 | Courier Prime, Cutive Mono; Special Elite ở thư mục `apache` |
| mono/serif_mono | 2 | Xanh Mono, Libertinus Mono |

Test `every_subclass_has_five_families_except_the_documented_gaps` bắt buộc danh sách này khớp bảng.

## Sửa ví dụ trong taxonomy mục 4 (đối chiếu `tags/all/families.csv`)

| Ví dụ trong taxonomy | Thực tế |
|---|---|
| Merriweather = Transitional "theo Google" | Không có tag hình dáng nào; không dùng |
| IM Fell | Không có tag hình dáng; không dùng |
| Cardo = venetian_oldstyle | GF: Old Style Garalde → ô `garalde` |
| Cinzel = glyphic | GF gán Old Style Garalde; thiết kế là glyphic (gán tay, visual) |
| Marcellus (glyphic) | Không có tag; glyphic (gán tay, visual) |
| Alfa Slab One = clarendon/egyptian | GF: Fat Face + Slab Geometric + Slab Humanist; không dùng (display fat face) |
| Roboto Slab, Special Elite, Yellowtail, Satisfy, Permanent Marker, Rock Salt | Nằm ở thư mục `apache` (không OFL) → loại khỏi bảng |
| Zilla Slab, Aleo = egyptian | GF: Slab Humanist → `humanist_slab` |
| Crete Round = humanist slab | GF: Slab Clarendon |
| Bitter = humanist slab | GF gán Serif/Scotch; thiết kế là slab → gán tay, weak |
| Albert Sans = neo_grotesque | GF: Geometric |
| Nunito Sans = humanist | GF: Neo Grotesque + Rounded |
| Orbitron = squarish | GF chỉ có Theme/Techno; squarish dùng Exo 2, Michroma, Saira, Aldrich, Bai Jamjuree, Blinker, Rajdhani |
| Belleza = Sans/Glyphic | GF: Sans/Humanist (vẫn xếp `flared`, gán tay, visual) |
| Federo = flared | GF: Theme/Art Deco (xếp `flared`, gán tay, visual) |
| Gochi Hand = childlike | GF: Script/Handwritten (xếp childlike, weak) |
| Bungee Shade = shaded | GF không gắn Shaded (gán tay, giữ trong `shaded_3d`) |

## Baseline hiện tại (classifier cũ, ánh xạ sang taxonomy v2)

Xem `baseline_old_classifier.md`. Điểm cần nhớ khi so sánh:
- Ánh xạ: `humanist_oldstyle` → `venetian_oldstyle` (nên `garalde` luôn tính sai), `serif/slab` → class `slab` không subclass, `script/handwritten` → `handwriting/casual_print`, trait stencil/inline_outline/pixel → Display.
- Classifier cũ không có `Handwriting`, `Display` (chỉ qua trait), `shaded_3d`, `flared`, `squarish`, `signature`…: các ô này ở baseline gần như bằng 0 là do thiếu loại, không phải do đo sai.
