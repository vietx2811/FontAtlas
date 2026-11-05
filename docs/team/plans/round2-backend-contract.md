# Round 2 backend contract — 2026-11-06 (draft, implementation follows)

## Similar theo family

New async command `similar_families({ face: FaceKeyDto, opts: SimilarOptsDto, q: FaceQueryDto }) -> SimilarFamilyPageDto`.

- Existing `similar` remains compatible (face results). New command removes every face whose typographic family equals the source, before ranking; groups by exact typographic family (fallback family), not superfamily or fuzzy spelling.
- `SimilarFamilyPageDto = { total: number, offset: number, items: SimilarFamilyDto[] }`; `total` counts comparable families after filters. `SimilarFamilyDto = { family: string, representative: SimilarHitDto, faces: FaceKeyDto[], count: number }`.
- Representative = nearest comparable face passing all filters; tie by FaceKey. Members = all indexed faces of that family passing filters, sorted by FaceKey. Counts exclude missing by default. Order stable across pages for an unchanged index/query, including exact image reranking. No fixed top-face pool that can omit large families.
- `q.offset`, `q.limit` count families (default 12, max 100). `q.sort/descending` and `opts.k` ignored, similarity order wins. Every existing metadata/user filter in `FaceQueryDto` applies through the real Store query (text, vi, license, variable, italic, sourceIds, favoritesOnly, collectionId, tagIds, includeMissing, faceKeys); existing axisFilters/hard remain supported. Legacy opts.viOnly/sourceIds/weightRange intersect q. Image-only fallback applies when source lacks axes and useImage is true; otherwise not_comparable.

## Raw domains and filters

New `RawAxisRangeDto = { axis: string, range: [number, number] }`, optional-on-input/default-empty `FaceQueryDto.rawAxisRanges` array. AND, inclusive physical values, invalid/unknown/nonfinite/reversed -> invalid_arg. Missing/failed values do not match. No change to existing axisFilters semantics (reference percentiles/classes, not physical units).

New async `axis_stats({ q: FaceQueryDto }) -> AxisStatsDto[]` for all 16 index axes, in index order. `q` metadata/user filters apply, paging/sort ignored; `axisFilters`, `rawAxisRanges`, `hard` ignored to keep sliders from narrowing themselves. Stats describe indexed faces, not unindexed files. `AxisStatsDto = { axis: string, unit: string, count: number, min: number|null, max: number|null, p1: number|null, p99: number|null }`.

Finite readings with measured/estimated quality only. Empty -> count 0, four nulls; one/constant -> all four equal. Quantiles linear interpolation at (n-1)*p; full min/max always retained so UI can reach outliers beyond p1/p99. No fabricated padding or rounding. For stem, only default lowercase method is included: uppercase fallback divides by cap-height, a different denominator.

Units (raw, no conversion to %): x_height_em/ascender_em = `em`; x_height_cap = `cap_height_ratio`; stem/aperture_e/width/mean_advance/spacing = `x_height_ratio`; stress_deg/slant_deg/aperture = `deg`; darkness/counter = `area_ratio`; contrast/width_cv/serif_foot = `ratio`. UI can label exact denominator and optionally multiply ratios by 100 for display, but must convert back before query.

## Model download

Existing commands/events preserved: model_download, model_cancel, model_redownload, model_status. bytesPerSec is byte/s; decimal MB/s = /1,000,000. done/total are transport bytes (header+vision for Range, full source size for fallback), not always local model bytes. Resumed reflects verified HTTP 206 continuation. Bundled manifest/revision is trust anchor; verify segment SHA and entire generated local file SHA before Ready. No third-party mirror/config mutation/large download.

Investigate maintained official OSS options before choosing implementation. Retain existing ureq/extraction unless another library handles pinned partial safetensors + trusted checksum with less complexity. Bound connections, preserve cancel/resume/retry, validate exact Content-Range, fall back to single stream for Range unsupported. Reliability tests use tiny localhost fixtures.

## Measurement caveats

No analyzer algorithm changes. Claude's stress=0 and Source Code Pro g-storeys notes remain unverified hypotheses for Lead/domain validation. This work exposes current measured values and units; does not certify their scientific validity.

## Addendum approved by Lead

`AxisReadingDto` gains `rawValue: number|null`, `unit: string|null`. Existing `value` stays compatible (serif_foot class string); numeric serif_foot remains available as rawValue. Quality/reason/method unchanged. For caps fallback stem unit is cap_height_ratio. Detail axes now include cap_height_em and descender_em read directly from Anatomy; descender retains its negative sign. These two extra measurements are not in the 16-dimensional Index/stats/filter API. Maximum rawAxisRanges length: 16.
