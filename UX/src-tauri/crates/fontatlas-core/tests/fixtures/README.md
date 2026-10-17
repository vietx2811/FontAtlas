# Fixture font

Mọi fixture là font mã nguồn mở. Giấy phép đọc từ bảng `name` (nameID 13/14) bằng chính `fontatlas-core`.

| File | Nguồn | Giấy phép | Kích thước (byte) | sha256 |
|---|---|---|---|---|
| `roboto.ttf` | `fonts/samples/` của repo (Plan 1); không rõ SHA google/fonts (Plan 1) | OFL-1.1 | 112 796 | `86955f053abee377e24691de4bd63bd3ee7aefcf2b6f7cbcb023175e93718fbc` |
| `open_sans.ttf` | `fonts/samples/` (Plan 1); không rõ SHA (Plan 1) | OFL-1.1 | 122 116 | `47ed1cfaa7ffca04a04ab850b2938fd52c09d1b44cfa3e2980878e166b0cbd87` |
| `playfair_display.ttf` | `fonts/samples/` (Plan 1); không rõ SHA (Plan 1) | OFL-1.1 | 123 240 | `1609a54fa3ad1abc8d0037132f73700333241ea0b65672079998e352f151dea7` |
| `libre_baskerville.ttf` | `fonts/samples/` (Plan 1); không rõ SHA (Plan 1) | OFL-1.1 | 106 728 | `f5bc4341f15de5e877d8d95b6b14b33e9a3da1f8fef4ed0700ed407f096cffb6` |
| `source_code_pro.ttf` | `fonts/samples/` (Plan 1); không rõ SHA (Plan 1) | OFL-1.1 | 121 080 | `66a11c481af5bdcdb91a72282bdb0b9bb5992208a72d89e1dd393e5e8255c7a2` |
| `open_sans.woff` | sinh từ `open_sans.ttf` bằng `make_woff.py` (fontTools) | OFL-1.1 | — | `dda07ff4117fcba23a7944947fc006d71cbc6accef130947c93445bd18cec549` |
| `open_sans.woff2` | sinh từ `open_sans.ttf` bằng `make_woff.py` (fontTools + brotli) | OFL-1.1 | — | `2033666bcd1a75ef6d3b344bd35a2b40410fe6d89843df41b615c0d0bc0107f7` |
| `public_sans_vf.ttf` | `google/fonts@47d54ca1ebb14fd864695f1d39dc734f39017c33:ofl/publicsans/PublicSans[wght].ttf` | OFL-1.1 | 103 316 | `d75a7dc1a27eb9e336d5b33f55489d2ecb5621bf694d5c43b2415bce2ca830a8` |

`public_sans_vf.ttf` là variable font có trục `wght` 100–900 và hỗ trợ tiếng Việt. Instance mặc định của nó là Thin (wght 100).
