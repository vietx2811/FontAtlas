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

## Accuracy regressions (core-0.4.0)

Unmodified OFL fonts from `google/fonts@47d54ca1ebb14fd864695f1d39dc734f39017c33`; license texts in `accuracy-licenses/`. System/commercial fonts are not redistributed.

| Fixture | Source under ofl/ | Bytes | SHA-256 |
|---|---|---:|---|
| `roboto_italic_vf.ttf` | `roboto/Roboto-Italic[wdth,wght].ttf` | 530944 | `9725a847af6b460ffca162ae66d20dad48b01876137947180b42d7dcd7887182` |
| `fira_sans_regular.ttf` | `firasans/FiraSans-Regular.ttf` | 456996 | `c29556a2719bf613ef3d5e070e40d903a8965d9c081beca1375dc1e6e0f93c23` |
| `bebas_neue.ttf` | `bebasneue/BebasNeue-Regular.ttf` | 61400 | `08e4623805102d819f58601e46e345648846075e363b2ceb23313c2d1c83ec73` |
| `oswald_vf.ttf` | `oswald/Oswald[wght].ttf` | 172088 | `5b38c246e255a12f5712d640d56bcced0472466fc68983d2d0410ec0457c2817` |
| `miltonian.ttf` | `miltonian/Miltonian-Regular.ttf` | 126804 | `bd4ada3764e6774964efe097463533b2759755e390f68e3d16974264f8dd2d98` |
| `libre_baskerville_italic_vf.ttf` | `librebaskerville/LibreBaskerville-Italic[wght].ttf` | 167456 | `223959683dc73ec4437bd61fabaa4b3f22209e22855ffd3aee36ba61a5116e97` |
| `dancing_script_vf.ttf` | `dancingscript/DancingScript[wght].ttf` | 133636 | `21808625578fe8d8cd10cb684be546dca077b27cd03a53a2f1ec11dc743c924c` |
