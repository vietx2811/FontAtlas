/** Tên tiếng Anh của thuật ngữ typo + ánh xạ khoá backend → khoá thuật ngữ (nguồn duy nhất, không phụ thuộc React). */
export const TERM_NAMES: Record<string, string> = {
  x_height: 'x-height', cap_height: 'cap height', ascender: 'ascender', descender: 'descender', stem: 'stem weight', contrast: 'contrast ⊥', contrast_h: 'contrast H',
  stress: 'stress', slant: 'slant I/T', aperture: 'aperture c', aperture_e: 'aperture e', serif_foot: 'serif foot', width: 'width o', counter: 'counter o', spacing: 'letter spacing',
  darkness: 'ink density', x_height_cap: 'x / cap ratio', mean_advance: 'mean advance', width_cv: 'advance variation', terminal: 'terminal c', a_storeys: 'a storeys',
  scope_session: 'session scope', scope_persistent: 'persistent scope',
  dataless: 'Dataless file', always_keep: 'Always keep on this device',
  g_storeys: 'g storeys', weight_class: 'Weight', width_class: 'Width', overlay: 'Overlay', activate: 'Active', vi_marks: 'Stacked diacritics', baseline: 'baseline', classification: 'Classification',
  'class.serif': 'Serif', 'class.sans': 'Sans', 'class.mono': 'Mono', 'class.script': 'Script', 'class.blackletter': 'Blackletter', 'class.symbol': 'Symbol', 'class.uncertain': 'Uncertain',
  'sub.old_style': 'Old style', 'sub.transitional': 'Transitional', 'sub.didone': 'Didone', 'sub.slab': 'Slab', 'sub.geometric': 'Geometric', 'sub.humanist': 'Humanist', 'sub.grotesque': 'Grotesque', 'sub.neo_grotesque': 'Neo-grotesque',
  hybrid: 'Hybrid',
  variable_font: 'Variable', static_font: 'Static', font_collection: 'TTC / OTC', opentype: 'OpenType (OTF)', truetype: 'TrueType (TTF)', revision: 'Revision', glyph: 'Glyph',
  auto_activation: 'Auto-activation', beta: 'Beta', postscript_name: 'PostScript name',
  'class.slab': 'Slab',
  'class.handwriting': 'Handwriting',
  'class.display': 'Display',
  'sub.venetian_oldstyle': 'Venetian oldstyle',
  'sub.garalde': 'Garalde',
  'sub.glyphic': 'Glyphic',
  'sub.wedge': 'Wedge',
  'sub.clarendon': 'Clarendon',
  'sub.egyptian': 'Egyptian',
  'sub.humanist_slab': 'Humanist slab',
  'sub.flared': 'Flared',
  'sub.squarish': 'Squarish',
  'sub.rounded': 'Rounded',
  'sub.sans_mono': 'Sans mono',
  'sub.serif_mono': 'Serif mono',
  'sub.typewriter': 'Typewriter',
  'sub.coding': 'Coding',
  'sub.formal_copperplate': 'Formal copperplate',
  'sub.brush': 'Brush',
  'sub.monoline_script': 'Monoline script',
  'sub.upright_script': 'Upright script',
  'sub.casual_print': 'Casual print',
  'sub.marker': 'Marker',
  'sub.stencil': 'Stencil',
  'sub.inline_outline': 'Inline / outline',
  'sub.pixel_bitmap': 'Pixel / bitmap',
  'sub.shaded_3d': 'Shaded 3D',
};

/** Khoá số đo/trục từ backend → khoá thuật ngữ. */
const ALIAS: Record<string, string> = {
  x_height_em: 'x_height', cap_height_em: 'cap_height', ascender_em: 'ascender', descender_em: 'descender', stress_deg: 'stress', slant_deg: 'slant',
  slant_i_t: 'slant', weight: 'stem', serif: 'serif_foot',
};
export const termKey = (k: string): string => ALIAS[k] ?? k;
/** Tên EN dùng chung cho mọi màn (Compare, Giải phẫu, Pair, bộ lọc): cùng một thuật ngữ, cùng một tên. */
export const termName = (k: string): string => TERM_NAMES[termKey(k)] ?? k;
export const classTermKey = (c: string): string => `class.${c}`;
export const subclassTermKey = (s: string): string => `sub.${s.toLowerCase().replace(/[\s-]+/g, '_')}`;
