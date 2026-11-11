## Taxonomy v2 — baseline classifier hiện tại (ánh xạ class cũ → mới)

224 họ có kết quả, 0 thiếu font/METADATA, 0 không phân tích được.

## Split `test` (122 họ)

### Class — 106 họ (nhãn gf/visual)

| đúng | sai | Uncertain | đúng/tổng | đúng/đã gán |
|---|---|---|---|---|
| 79 | 7 | 20 | 74.5% | 91.9% |

Hàng = đáp án, cột = dự đoán.

| đáp án \ dự đoán | serif | slab | sans | script | handwriting | mono | display | blackletter | symbol | uncertain |
|---|---|---|---|---|---|---|---|---|---|---|
| serif | 17 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| slab | 1 | 6 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| sans | 1 | 0 | 31 | 0 | 0 | 0 | 0 | 0 | 0 | 5 |
| script | 1 | 0 | 1 | 14 | 0 | 0 | 0 | 0 | 0 | 1 |
| handwriting | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 3 |
| mono | 0 | 0 | 0 | 0 | 0 | 6 | 0 | 0 | 0 | 1 |
| display | 1 | 0 | 1 | 0 | 0 | 1 | 4 | 0 | 0 | 7 |
| blackletter | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 2 |

### Subclass (nhãn gf/visual; mỗi họ tính theo từng subclass của nó)

| class/subclass | số họ | đúng | nhầm sang |
|---|---|---|---|
| display/inline_outline | 6 | 2 | uncertain×3, serif/transitional×1 |
| display/pixel_bitmap | 5 | 1 | uncertain×3, mono/sans_mono×1 |
| display/stencil | 3 | 1 | sans/geometric×1, uncertain×1 |
| handwriting/casual_print | 3 | 0 | uncertain×3 |
| mono/sans_mono | 3 | 2 | serif_mono×1 |
| mono/serif_mono | 2 | 1 | sans_mono×1 |
| mono/typewriter | 2 | 1 | uncertain×1 |
| sans/flared | 5 | 0 | humanist×3, serif/glyphic×1, uncertain×1 |
| sans/geometric | 6 | 5 | uncertain×1 |
| sans/grotesque | 3 | 3 |  |
| sans/humanist | 4 | 4 |  |
| sans/neo_grotesque | 6 | 2 | geometric×2, grotesque×2 |
| sans/rounded | 5 | 1 | humanist×2, geometric×1, uncertain×1 |
| sans/squarish | 6 | 0 | humanist×4, geometric×1, grotesque×1 |
| script/brush | 4 | 2 | formal_copperplate×1, upright_script×1 |
| script/calligraphic_unjoined | 4 | 0 | formal_copperplate×1, sans/geometric×1, serif/didone×1, uncertain×1 |
| script/formal_copperplate | 6 | 4 | brush×1, upright_script×1 |
| script/monoline_script | 3 | 1 | formal_copperplate×2 |
| serif/didone | 5 | 4 | venetian_oldstyle×1 |
| serif/garalde | 3 | 0 | venetian_oldstyle×2, transitional×1 |
| serif/glyphic | 4 | 2 | uncertain×1, venetian_oldstyle×1 |
| serif/transitional | 4 | 4 |  |
| serif/venetian_oldstyle | 2 | 0 | garalde×2 |
| slab/clarendon | 1 | 0 | serif/transitional×1 |
| slab/egyptian | 2 | 2 |  |
| slab/humanist_slab | 4 | 0 | egyptian×3, clarendon×1 |

#### serif: đáp án subclass × dự đoán

| đáp án \ dự đoán | didone | garalde | glyphic | transitional | uncertain | venetian_oldstyle |
|---|---|---|---|---|---|---|
| didone | 4 | 0 | 0 | 0 | 0 | 1 |
| garalde | 0 | 0 | 0 | 1 | 0 | 2 |
| glyphic | 0 | 0 | 2 | 0 | 1 | 1 |
| transitional | 0 | 0 | 0 | 4 | 0 | 0 |
| venetian_oldstyle | 0 | 2 | 0 | 0 | 0 | 0 |

#### slab: đáp án subclass × dự đoán

| đáp án \ dự đoán | clarendon | egyptian | serif/transitional |
|---|---|---|---|
| clarendon | 0 | 0 | 1 |
| egyptian | 0 | 2 | 0 |
| humanist_slab | 1 | 3 | 0 |

#### sans: đáp án subclass × dự đoán

| đáp án \ dự đoán | geometric | grotesque | humanist | neo_grotesque | rounded | serif/glyphic | uncertain |
|---|---|---|---|---|---|---|---|
| flared | 0 | 0 | 3 | 0 | 0 | 1 | 1 |
| geometric | 5 | 0 | 0 | 0 | 0 | 0 | 1 |
| grotesque | 0 | 3 | 0 | 0 | 0 | 0 | 0 |
| humanist | 0 | 0 | 4 | 0 | 0 | 0 | 0 |
| neo_grotesque | 2 | 2 | 0 | 2 | 0 | 0 | 0 |
| rounded | 1 | 0 | 2 | 0 | 1 | 0 | 1 |
| squarish | 1 | 1 | 4 | 0 | 0 | 0 | 0 |

#### script: đáp án subclass × dự đoán

| đáp án \ dự đoán | brush | formal_copperplate | monoline_script | sans/geometric | serif/didone | uncertain | upright_script |
|---|---|---|---|---|---|---|---|
| brush | 2 | 1 | 0 | 0 | 0 | 0 | 1 |
| calligraphic_unjoined | 0 | 1 | 0 | 1 | 1 | 1 | 0 |
| formal_copperplate | 1 | 4 | 0 | 0 | 0 | 0 | 1 |
| monoline_script | 0 | 2 | 1 | 0 | 0 | 0 | 0 |

#### handwriting: đáp án subclass × dự đoán

| đáp án \ dự đoán | uncertain |
|---|---|
| casual_print | 3 |

#### mono: đáp án subclass × dự đoán

| đáp án \ dự đoán | sans_mono | serif_mono | typewriter | uncertain |
|---|---|---|---|---|
| sans_mono | 2 | 1 | 0 | 0 |
| serif_mono | 1 | 1 | 0 | 0 |
| typewriter | 0 | 0 | 1 | 1 |

#### display: đáp án subclass × dự đoán

| đáp án \ dự đoán | inline_outline | mono/sans_mono | pixel_bitmap | sans/geometric | serif/transitional | stencil | uncertain |
|---|---|---|---|---|---|---|---|
| inline_outline | 2 | 0 | 0 | 0 | 1 | 0 | 3 |
| pixel_bitmap | 0 | 1 | 1 | 0 | 0 | 0 | 3 |
| stencil | 0 | 0 | 0 | 1 | 0 | 1 | 1 |

### Fixture ranh giới (class đúng?)

| nhóm | họ | class đáp án | class dự đoán | đúng |
|---|---|---|---|---|
| small_serif_risk | Marcellus | serif | serif | ✓ |
| small_serif_risk | Marcellus SC | serif | serif | ✓ |
| tailed_l_sans | Port Lligat Sans | sans | uncertain | ✗ |
| tailed_l_sans | Ruluko | sans | uncertain | ✗ |

### Nhãn weak (không cổng, chỉ tham khảo)

| nhãn | số họ | class đúng | subclass đúng |
|---|---|---|---|
| weak | 16 | 6 | 3/16 |

## Split `tune` (102 họ)

### Class — 89 họ (nhãn gf/visual)

| đúng | sai | Uncertain | đúng/tổng | đúng/đã gán |
|---|---|---|---|---|
| 61 | 6 | 22 | 68.5% | 91.0% |

Hàng = đáp án, cột = dự đoán.

| đáp án \ dự đoán | serif | slab | sans | script | handwriting | mono | display | blackletter | symbol | uncertain |
|---|---|---|---|---|---|---|---|---|---|---|
| serif | 19 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| slab | 1 | 11 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| sans | 0 | 0 | 17 | 0 | 0 | 0 | 0 | 0 | 0 | 2 |
| script | 1 | 0 | 0 | 5 | 0 | 0 | 0 | 0 | 0 | 3 |
| handwriting | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 3 |
| mono | 0 | 0 | 0 | 0 | 0 | 5 | 0 | 0 | 0 | 0 |
| display | 0 | 0 | 3 | 0 | 0 | 0 | 1 | 0 | 0 | 10 |
| blackletter | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 | 0 | 2 |

### Subclass (nhãn gf/visual; mỗi họ tính theo từng subclass của nó)

| class/subclass | số họ | đúng | nhầm sang |
|---|---|---|---|
| display/inline_outline | 1 | 0 | uncertain×1 |
| display/pixel_bitmap | 2 | 0 | uncertain×2 |
| display/shaded_3d | 7 | 0 | uncertain×6, sans/geometric×1 |
| display/stencil | 4 | 1 | sans/geometric×1, sans/humanist×1, uncertain×1 |
| handwriting/casual_print | 5 | 2 | uncertain×3 |
| mono/sans_mono | 5 | 5 |  |
| sans/flared | 1 | 0 | humanist×1 |
| sans/geometric | 2 | 2 |  |
| sans/grotesque | 5 | 3 | geometric×1, neo_grotesque×1 |
| sans/humanist | 4 | 4 |  |
| sans/neo_grotesque | 1 | 0 | grotesque×1 |
| sans/rounded | 3 | 0 | geometric×3 |
| sans/squarish | 1 | 0 | geometric×1 |
| script/brush | 2 | 0 | formal_copperplate×1, uncertain×1 |
| script/calligraphic_unjoined | 1 | 0 | serif/venetian_oldstyle×1 |
| script/formal_copperplate | 2 | 1 | uncertain×1 |
| script/monoline_script | 4 | 0 | formal_copperplate×2, uncertain×1, upright_script×1 |
| serif/didone | 3 | 3 |  |
| serif/garalde | 5 | 0 | venetian_oldstyle×5 |
| serif/glyphic | 2 | 1 | garalde×1 |
| serif/transitional | 4 | 3 | venetian_oldstyle×1 |
| serif/venetian_oldstyle | 6 | 1 | garalde×2, didone×1, transitional×1, uncertain×1 |
| slab/clarendon | 5 | 3 | egyptian×1, uncertain×1 |
| slab/egyptian | 4 | 4 |  |
| slab/humanist_slab | 4 | 0 | egyptian×2, clarendon×1, serif/garalde×1 |

#### serif: đáp án subclass × dự đoán

| đáp án \ dự đoán | didone | garalde | glyphic | transitional | uncertain | venetian_oldstyle |
|---|---|---|---|---|---|---|
| didone | 3 | 0 | 0 | 0 | 0 | 0 |
| garalde | 0 | 0 | 0 | 0 | 0 | 5 |
| glyphic | 0 | 1 | 1 | 0 | 0 | 0 |
| transitional | 0 | 0 | 0 | 3 | 0 | 1 |
| venetian_oldstyle | 1 | 2 | 0 | 1 | 1 | 1 |

#### slab: đáp án subclass × dự đoán

| đáp án \ dự đoán | clarendon | egyptian | serif/garalde | uncertain |
|---|---|---|---|---|
| clarendon | 3 | 1 | 0 | 1 |
| egyptian | 0 | 4 | 0 | 0 |
| humanist_slab | 1 | 2 | 1 | 0 |

#### sans: đáp án subclass × dự đoán

| đáp án \ dự đoán | geometric | grotesque | humanist | neo_grotesque |
|---|---|---|---|---|
| flared | 0 | 0 | 1 | 0 |
| geometric | 2 | 0 | 0 | 0 |
| grotesque | 1 | 3 | 0 | 1 |
| humanist | 0 | 0 | 4 | 0 |
| neo_grotesque | 0 | 1 | 0 | 0 |
| rounded | 3 | 0 | 0 | 0 |
| squarish | 1 | 0 | 0 | 0 |

#### script: đáp án subclass × dự đoán

| đáp án \ dự đoán | formal_copperplate | serif/venetian_oldstyle | uncertain | upright_script |
|---|---|---|---|---|
| brush | 1 | 0 | 1 | 0 |
| calligraphic_unjoined | 0 | 1 | 0 | 0 |
| formal_copperplate | 1 | 0 | 1 | 0 |
| monoline_script | 2 | 0 | 1 | 1 |

#### handwriting: đáp án subclass × dự đoán

| đáp án \ dự đoán | casual_print | uncertain |
|---|---|---|
| casual_print | 2 | 3 |

#### mono: đáp án subclass × dự đoán

| đáp án \ dự đoán | sans_mono |
|---|---|
| sans_mono | 5 |

#### display: đáp án subclass × dự đoán

| đáp án \ dự đoán | sans/geometric | sans/humanist | stencil | uncertain |
|---|---|---|---|---|
| inline_outline | 0 | 0 | 0 | 1 |
| pixel_bitmap | 0 | 0 | 0 | 2 |
| shaded_3d | 1 | 0 | 0 | 6 |
| stencil | 1 | 1 | 1 | 1 |

### Fixture ranh giới (class đúng?)

| nhóm | họ | class đáp án | class dự đoán | đúng |
|---|---|---|---|---|
| tailed_l_sans | Bellota | sans | uncertain | ✗ |
| tailed_l_sans | Mooli | sans | uncertain | ✗ |

### Nhãn weak (không cổng, chỉ tham khảo)

| nhãn | số họ | class đúng | subclass đúng |
|---|---|---|---|
| weak | 13 | 11 | 0/13 |

## Split `all` (224 họ)

### Class — 195 họ (nhãn gf/visual)

| đúng | sai | Uncertain | đúng/tổng | đúng/đã gán |
|---|---|---|---|---|
| 140 | 13 | 42 | 71.8% | 91.5% |

Hàng = đáp án, cột = dự đoán.

| đáp án \ dự đoán | serif | slab | sans | script | handwriting | mono | display | blackletter | symbol | uncertain |
|---|---|---|---|---|---|---|---|---|---|---|
| serif | 36 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 |
| slab | 2 | 17 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |
| sans | 1 | 0 | 48 | 0 | 0 | 0 | 0 | 0 | 0 | 7 |
| script | 2 | 0 | 1 | 19 | 0 | 0 | 0 | 0 | 0 | 4 |
| handwriting | 0 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 0 | 6 |
| mono | 0 | 0 | 0 | 0 | 0 | 11 | 0 | 0 | 0 | 1 |
| display | 1 | 0 | 4 | 0 | 0 | 1 | 5 | 0 | 0 | 17 |
| blackletter | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 2 | 0 | 4 |

### Subclass (nhãn gf/visual; mỗi họ tính theo từng subclass của nó)

| class/subclass | số họ | đúng | nhầm sang |
|---|---|---|---|
| display/inline_outline | 7 | 2 | uncertain×4, serif/transitional×1 |
| display/pixel_bitmap | 7 | 1 | uncertain×5, mono/sans_mono×1 |
| display/shaded_3d | 7 | 0 | uncertain×6, sans/geometric×1 |
| display/stencil | 7 | 2 | sans/geometric×2, uncertain×2, sans/humanist×1 |
| handwriting/casual_print | 8 | 2 | uncertain×6 |
| mono/sans_mono | 8 | 7 | serif_mono×1 |
| mono/serif_mono | 2 | 1 | sans_mono×1 |
| mono/typewriter | 2 | 1 | uncertain×1 |
| sans/flared | 6 | 0 | humanist×4, serif/glyphic×1, uncertain×1 |
| sans/geometric | 8 | 7 | uncertain×1 |
| sans/grotesque | 8 | 6 | geometric×1, neo_grotesque×1 |
| sans/humanist | 8 | 8 |  |
| sans/neo_grotesque | 7 | 2 | grotesque×3, geometric×2 |
| sans/rounded | 8 | 1 | geometric×4, humanist×2, uncertain×1 |
| sans/squarish | 7 | 0 | humanist×4, geometric×2, grotesque×1 |
| script/brush | 6 | 2 | formal_copperplate×2, uncertain×1, upright_script×1 |
| script/calligraphic_unjoined | 5 | 0 | formal_copperplate×1, sans/geometric×1, serif/didone×1, serif/venetian_oldstyle×1, uncertain×1 |
| script/formal_copperplate | 8 | 5 | brush×1, uncertain×1, upright_script×1 |
| script/monoline_script | 7 | 1 | formal_copperplate×4, uncertain×1, upright_script×1 |
| serif/didone | 8 | 7 | venetian_oldstyle×1 |
| serif/garalde | 8 | 0 | venetian_oldstyle×7, transitional×1 |
| serif/glyphic | 6 | 3 | garalde×1, uncertain×1, venetian_oldstyle×1 |
| serif/transitional | 8 | 7 | venetian_oldstyle×1 |
| serif/venetian_oldstyle | 8 | 1 | garalde×4, didone×1, transitional×1, uncertain×1 |
| slab/clarendon | 6 | 3 | egyptian×1, serif/transitional×1, uncertain×1 |
| slab/egyptian | 6 | 6 |  |
| slab/humanist_slab | 8 | 0 | egyptian×5, clarendon×2, serif/garalde×1 |

#### serif: đáp án subclass × dự đoán

| đáp án \ dự đoán | didone | garalde | glyphic | transitional | uncertain | venetian_oldstyle |
|---|---|---|---|---|---|---|
| didone | 7 | 0 | 0 | 0 | 0 | 1 |
| garalde | 0 | 0 | 0 | 1 | 0 | 7 |
| glyphic | 0 | 1 | 3 | 0 | 1 | 1 |
| transitional | 0 | 0 | 0 | 7 | 0 | 1 |
| venetian_oldstyle | 1 | 4 | 0 | 1 | 1 | 1 |

#### slab: đáp án subclass × dự đoán

| đáp án \ dự đoán | clarendon | egyptian | serif/garalde | serif/transitional | uncertain |
|---|---|---|---|---|---|
| clarendon | 3 | 1 | 0 | 1 | 1 |
| egyptian | 0 | 6 | 0 | 0 | 0 |
| humanist_slab | 2 | 5 | 1 | 0 | 0 |

#### sans: đáp án subclass × dự đoán

| đáp án \ dự đoán | geometric | grotesque | humanist | neo_grotesque | rounded | serif/glyphic | uncertain |
|---|---|---|---|---|---|---|---|
| flared | 0 | 0 | 4 | 0 | 0 | 1 | 1 |
| geometric | 7 | 0 | 0 | 0 | 0 | 0 | 1 |
| grotesque | 1 | 6 | 0 | 1 | 0 | 0 | 0 |
| humanist | 0 | 0 | 8 | 0 | 0 | 0 | 0 |
| neo_grotesque | 2 | 3 | 0 | 2 | 0 | 0 | 0 |
| rounded | 4 | 0 | 2 | 0 | 1 | 0 | 1 |
| squarish | 2 | 1 | 4 | 0 | 0 | 0 | 0 |

#### script: đáp án subclass × dự đoán

| đáp án \ dự đoán | brush | formal_copperplate | monoline_script | sans/geometric | serif/didone | serif/venetian_oldstyle | uncertain | upright_script |
|---|---|---|---|---|---|---|---|---|
| brush | 2 | 2 | 0 | 0 | 0 | 0 | 1 | 1 |
| calligraphic_unjoined | 0 | 1 | 0 | 1 | 1 | 1 | 1 | 0 |
| formal_copperplate | 1 | 5 | 0 | 0 | 0 | 0 | 1 | 1 |
| monoline_script | 0 | 4 | 1 | 0 | 0 | 0 | 1 | 1 |

#### handwriting: đáp án subclass × dự đoán

| đáp án \ dự đoán | casual_print | uncertain |
|---|---|---|
| casual_print | 2 | 6 |

#### mono: đáp án subclass × dự đoán

| đáp án \ dự đoán | sans_mono | serif_mono | typewriter | uncertain |
|---|---|---|---|---|
| sans_mono | 7 | 1 | 0 | 0 |
| serif_mono | 1 | 1 | 0 | 0 |
| typewriter | 0 | 0 | 1 | 1 |

#### display: đáp án subclass × dự đoán

| đáp án \ dự đoán | inline_outline | mono/sans_mono | pixel_bitmap | sans/geometric | sans/humanist | serif/transitional | stencil | uncertain |
|---|---|---|---|---|---|---|---|---|
| inline_outline | 2 | 0 | 0 | 0 | 0 | 1 | 0 | 4 |
| pixel_bitmap | 0 | 1 | 1 | 0 | 0 | 0 | 0 | 5 |
| shaded_3d | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 6 |
| stencil | 0 | 0 | 0 | 2 | 1 | 0 | 2 | 2 |

### Fixture ranh giới (class đúng?)

| nhóm | họ | class đáp án | class dự đoán | đúng |
|---|---|---|---|---|
| small_serif_risk | Marcellus | serif | serif | ✓ |
| small_serif_risk | Marcellus SC | serif | serif | ✓ |
| tailed_l_sans | Bellota | sans | uncertain | ✗ |
| tailed_l_sans | Mooli | sans | uncertain | ✗ |
| tailed_l_sans | Port Lligat Sans | sans | uncertain | ✗ |
| tailed_l_sans | Ruluko | sans | uncertain | ✗ |

### Nhãn weak (không cổng, chỉ tham khảo)

| nhãn | số họ | class đúng | subclass đúng |
|---|---|---|---|
| weak | 29 | 17 | 3/29 |
