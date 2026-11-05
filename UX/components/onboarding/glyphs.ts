// Khối chữ ở bước quét: chữ cái ngẫu nhiên (Latin thường và chữ Việt có dấu) vẽ bằng font hệ thống ngẫu nhiên. Có seed để test lặp lại được.

/** Chữ Latin không dấu, hình dạng khác nhau rõ. */
export const LATIN = 'ABCDEGHKMNRSTaegkrstyQRf';
/** Chữ Việt có dấu hoặc móc/gạch (một ký tự dựng sẵn mỗi chữ). */
export const VIET = 'ẶẫựƠơƯưĐđẮằẳỂệỔỜấầộỗợữừỵ';
/** Họ font có sẵn trên macOS (không đo được danh sách thật từ webview nên dùng bộ phổ biến). */
export const SYSTEM_FAMILIES = [
  'Helvetica Neue', 'Georgia', 'Times New Roman', 'Courier New', 'Palatino', 'Baskerville', 'Didot', 'Avenir Next', 'Futura', 'Optima',
  'Gill Sans', 'Menlo', 'Impact', 'Copperplate', 'Hoefler Text', 'American Typewriter', 'Rockwell', 'Verdana', 'Trebuchet MS', 'Bodoni 72', 'Cochin', 'Charter',
] as const;

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
function familyAvoiding(r: () => number, avoid: Array<string | undefined>): string {
  let f: string;
  do { f = pick(r, SYSTEM_FAMILIES); } while (avoid.includes(f));
  return f;
}

export function pickBlocks(seed: number, count = 9): Block[] {
  const r = rng(seed);
  const viet = Math.min(count - 3, Math.max(3, Math.floor(count / 2)));
  const kinds = Array.from({ length: count }, (_, i) => i < viet);
  for (let i = kinds.length - 1; i > 0; i--) { const j = Math.floor(r() * (i + 1)); [kinds[i], kinds[j]] = [kinds[j], kinds[i]]; }
  const out: Block[] = [];
  kinds.forEach((isViet, i) => {
    out.push({
      char: pick(r, isViet ? VIET : LATIN), viet: isViet, family: familyAvoiding(r, [out[i - 1]?.family]),
      delayMs: Math.floor(r() * 1100),
    });
  });
  return out;
}

/** Một nhịp: đổi chữ và font của 3 khối (cùng loại chữ để giữ tỉ lệ Latin/Việt), không trùng font với hai khối kề. */
export function rerollBlocks(blocks: Block[], seed: number, tick: number): Block[] {
  const r = rng(seed * 7919 + tick * 104729);
  const next = blocks.map(b => ({ ...b }));
  const idx = new Set<number>();
  while (idx.size < Math.min(3, blocks.length)) idx.add(Math.floor(r() * blocks.length));
  for (const i of idx) {
    const old = next[i];
    let char: string;
    do { char = pick(r, old.viet ? VIET : LATIN); } while (char === old.char);
    next[i] = { ...old, char, family: familyAvoiding(r, [old.family, next[i - 1]?.family, next[i + 1]?.family]) };
  }
  return next;
}
