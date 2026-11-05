// Khối chữ ở bước quét: chữ cái ngẫu nhiên (Latin thường và chữ Việt có dấu) vẽ bằng font hệ thống ngẫu nhiên. Có seed để test lặp lại được.
// Chỉ dùng font THỰC SỰ có trên máy (đo độ rộng so với họ dự phòng), đa dạng thể loại, mỗi họ tối đa 2 lần trong 9 khối,
// và chữ Việt chỉ đặt vào font có đủ glyph (nếu không WebView rơi về Times và mọi khối trông như nhau).

/** Chữ Latin không dấu, hình dạng khác nhau rõ. */
export const LATIN = 'ABCDEGHKMNRSTaegkrstyQRf';
/** Chữ Việt có dấu hoặc móc/gạch (một ký tự dựng sẵn mỗi chữ). */
export const VIET = 'ẶẫựƠơƯưĐđẮằẳỂệỔỜấầộỗợữừỵ';

export type Kind = 'serif' | 'sans' | 'mono' | 'script' | 'display';
export type Family = { name: string; kind: Kind; /** có đủ glyph tiếng Việt có dấu */ viet: boolean };
/** Họ font phổ biến trên macOS (webview không liệt kê được font cài đặt; danh sách này được lọc bằng isInstalled). */
export const FAMILIES: readonly Family[] = [
  { name: 'Helvetica Neue', kind: 'sans', viet: true }, { name: 'Avenir Next', kind: 'sans', viet: true }, { name: 'Verdana', kind: 'sans', viet: true },
  { name: 'Trebuchet MS', kind: 'sans', viet: true }, { name: 'Optima', kind: 'sans', viet: true }, { name: 'Gill Sans', kind: 'sans', viet: false }, { name: 'Futura', kind: 'sans', viet: false },
  { name: 'Georgia', kind: 'serif', viet: true }, { name: 'Times New Roman', kind: 'serif', viet: true }, { name: 'Palatino', kind: 'serif', viet: true },
  { name: 'Baskerville', kind: 'serif', viet: true }, { name: 'Didot', kind: 'serif', viet: true }, { name: 'Cochin', kind: 'serif', viet: true },
  { name: 'Charter', kind: 'serif', viet: true }, { name: 'Hoefler Text', kind: 'serif', viet: false }, { name: 'Bodoni 72', kind: 'serif', viet: false },
  { name: 'Courier New', kind: 'mono', viet: true }, { name: 'Menlo', kind: 'mono', viet: true }, { name: 'Andale Mono', kind: 'mono', viet: false },
  { name: 'Snell Roundhand', kind: 'script', viet: false }, { name: 'Apple Chancery', kind: 'script', viet: false }, { name: 'Brush Script MT', kind: 'script', viet: false },
  { name: 'Impact', kind: 'display', viet: false }, { name: 'Copperplate', kind: 'display', viet: false }, { name: 'Marker Felt', kind: 'display', viet: false },
  { name: 'Chalkduster', kind: 'display', viet: false }, { name: 'Rockwell', kind: 'display', viet: false },
];
export const SYSTEM_FAMILIES = FAMILIES.map(f => f.name);

// ---- Phát hiện font có thật -------------------------------------------------------------------------------------

export type Measure = (font: string) => number;
const BASES = ['monospace', 'serif', 'sans-serif'] as const;

/** Font thiếu thì trình duyệt rơi về họ dự phòng nên độ rộng bằng họ dự phòng; có thật thì lệch ít nhất một họ. */
export function isInstalled(name: string, measure: Measure): boolean {
  return BASES.some(b => measure(`"${name}", ${b}`) !== measure(b));
}
/** `measure === null` (không có canvas): không kiểm được nên coi mọi họ là có. */
export function installedFamilies(list: readonly Family[], measure: Measure | null): string[] {
  return list.filter(f => measure === null || isInstalled(f.name, measure)).map(f => f.name);
}
/** Đo bằng canvas 2D; trả null khi môi trường không có canvas (jsdom). */
export function canvasMeasure(): Measure | null {
  try {
    if (typeof document === 'undefined' || /jsdom/i.test(navigator.userAgent)) return null; // jsdom không có canvas
    const ctx = document.createElement('canvas').getContext?.('2d');
    if (!ctx) return null;
    return font => { ctx.font = `64px ${font}`; return ctx.measureText('mmmmmmmmmmlli ặẫ').width; };
  } catch { return null; }
}

// ---- Chọn khối --------------------------------------------------------------------------------------------------

export type Block = { char: string; family: string; viet: boolean; delayMs: number };

/** Bộ sinh số giả ngẫu nhiên nhỏ gọn (mulberry32): cùng seed cho cùng dãy. */
export function rng(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const pick = <T,>(r: () => number, list: ArrayLike<T>): T => list[Math.floor(r() * list.length)];

const MAX_REPEAT = 2;
const poolOf = (available?: readonly string[]): Family[] => {
  const p = available && available.length ? FAMILIES.filter(f => available.includes(f.name)) : [...FAMILIES];
  return p.length ? p : [...FAMILIES];
};

/**
 * Chọn họ font cho một khối: cần đủ glyph nếu là chữ Việt, khác họ hai khối kề, mỗi họ ≤ 2 lần,
 * và ưu tiên thể loại đang ít xuất hiện nhất (để có đủ serif/sans/mono/script/display). Hết lựa chọn thì nới dần từng ràng buộc.
 */
function chooseFamily(r: () => number, pool: Family[], needViet: boolean, used: Record<string, number>, kinds: Record<string, number>, avoid: Array<string | undefined>): Family {
  const tiers: Array<(f: Family) => boolean> = [
    f => (!needViet || f.viet) && !avoid.includes(f.name) && (used[f.name] ?? 0) < MAX_REPEAT,
    f => (!needViet || f.viet) && !avoid.includes(f.name),
    f => !needViet || f.viet,
    () => true,
  ];
  for (const ok of tiers) {
    const cand = pool.filter(ok);
    if (!cand.length) continue;
    const least = Math.min(...cand.map(f => kinds[f.kind] ?? 0));
    return pick(r, cand.filter(f => (kinds[f.kind] ?? 0) === least));
  }
  return pool[0];
}
const bump = (m: Record<string, number>, k: string, d: number) => { m[k] = (m[k] ?? 0) + d; };

export function pickBlocks(seed: number, count = 9, available?: readonly string[]): Block[] {
  const r = rng(seed);
  const pool = poolOf(available);
  const viet = Math.min(count - 3, Math.max(3, Math.floor(count / 2)));
  const isViet = Array.from({ length: count }, (_, i) => i < viet);
  for (let i = isViet.length - 1; i > 0; i--) { const j = Math.floor(r() * (i + 1)); [isViet[i], isViet[j]] = [isViet[j], isViet[i]]; }
  const used: Record<string, number> = {}; const kinds: Record<string, number> = {};
  const out: Block[] = [];
  isViet.forEach((v, i) => {
    const f = chooseFamily(r, pool, v, used, kinds, [out[i - 1]?.family]);
    bump(used, f.name, 1); bump(kinds, f.kind, 1);
    out.push({ char: pick(r, v ? VIET : LATIN), viet: v, family: f.name, delayMs: Math.floor(r() * 1100) });
  });
  return out;
}

/** Một nhịp: đổi chữ và font của 3 khối (cùng loại chữ để giữ tỉ lệ Latin/Việt), giữ nguyên các ràng buộc ở trên. */
export function rerollBlocks(blocks: Block[], seed: number, tick: number, available?: readonly string[]): Block[] {
  const r = rng(seed * 7919 + tick * 104729);
  const pool = poolOf(available);
  const next = blocks.map(b => ({ ...b }));
  const idx = new Set<number>();
  while (idx.size < Math.min(3, blocks.length)) idx.add(Math.floor(r() * blocks.length));
  const used: Record<string, number> = {}; const kinds: Record<string, number> = {};
  for (const b of next) { bump(used, b.family, 1); const k = FAMILIES.find(f => f.name === b.family)?.kind; if (k) bump(kinds, k, 1); }
  for (const i of idx) {
    const old = next[i];
    const oldKind = FAMILIES.find(f => f.name === old.family)?.kind;
    bump(used, old.family, -1); if (oldKind) bump(kinds, oldKind, -1);
    let char: string;
    do { char = pick(r, old.viet ? VIET : LATIN); } while (char === old.char);
    const f = chooseFamily(r, pool, old.viet, used, kinds, [old.family, next[i - 1]?.family, next[i + 1]?.family]);
    bump(used, f.name, 1); bump(kinds, f.kind, 1);
    next[i] = { ...old, char, family: f.name };
  }
  return next;
}
