// Logic thuần của splash: chọn font ngẫu nhiên từ thư viện (lọc), tỉ lệ x-height, nhịp đảo font. Không đụng DOM nên test được.
import type { FaceRow } from '../../features/library-grid/types';
import { MONO, SANS, SERIF, type WordmarkFont } from './wordmarkMetrics';

export const SERIF_PART = 'Font';
export const SANS_PART = 'Atlas';
export const WORD = `${SERIF_PART}${SANS_PART}`;
/** Mọi chữ cái của wordmark: font ngẫu nhiên phải có đủ các glyph này (ở đây là Latin cơ bản; kiểm bằng lọc ngôn ngữ `eng` đủ). */
export const NEEDED_GLYPHS = [...new Set(WORD)].join('');
export const SPLASH_MS = 2000;
export const REDUCED_HOLD_MS = 900;
export const MAX_USER_FONTS = 14;
export const MAX_FONT_BYTES = 6 * 1024 * 1024;

export const xRatio = (f: Pick<WordmarkFont, 'xHeight' | 'upm'>): number => f.xHeight / f.upm;
/** font-size (px) để x-height (px) của font có tỉ lệ `ratio` đạt `xHeightPx`. */
export const sizeForXHeight = (xHeightPx: number, ratio: number): number => xHeightPx / ratio;
/** Cỡ chữ của phần serif sao cho x-height bằng phần sans cỡ `sansSize`. */
export const serifSizeFor = (sansSize: number): number => sizeForXHeight(sansSize * xRatio(SANS), xRatio(SERIF));

export type PoolFont = { family: string; weight: number; ratio: number; user: boolean };
/** Font đóng gói (OFL) để đảo khi thư viện trống hoặc chưa nạp xong font người dùng. Tỉ lệ x-height đã đo. */
export const BUNDLED_POOL: readonly PoolFont[] = [
  { family: SERIF.family, weight: 300, ratio: xRatio(SERIF), user: false },
  { family: SERIF.family, weight: 800, ratio: xRatio(SERIF), user: false },
  { family: SANS.family, weight: 200, ratio: xRatio(SANS), user: false },
  { family: SANS.family, weight: 900, ratio: xRatio(SANS), user: false },
  { family: MONO.family, weight: 400, ratio: xRatio(MONO), user: false },
  { family: MONO.family, weight: 800, ratio: xRatio(MONO), user: false },
];

/** CHỈ nhận font nằm ở thư mục font hệ thống (chỉ đọc) hoặc của người dùng: chắc chắn là file thật trên đĩa. Mọi nơi khác (iCloud Desktop & Documents,
 *  Nextcloud, Synology Drive, OneDrive, Dropbox…) có thể là placeholder (dataless) mà việc đọc file sẽ kích hoạt tải về. Khi backend có cờ dataless
 *  (feat/cloud-fonts) sẽ đổi sang cờ đó và cho phép cả font cloud đã tải về; allowlist này vẫn là dự phòng khi thiếu cờ. */
export const SAFE_FONT_DIRS = /^(\/System\/Library\/Fonts\/|\/Library\/Fonts\/|\/Users\/[^/]+\/Library\/Fonts\/)/;
export const isSafeFontPath = (path: string): boolean => SAFE_FONT_DIRS.test(path) && !path.includes('/../');

type Cand = Pick<FaceRow, 'family' | 'faceIndex' | 'fileSize' | 'path' | 'missing'>;
/** Dùng được cho splash: có file trên đĩa (không mất; chỉ ở thư mục font hệ thống/người dùng), face 0 (TTC face ≥ 1 không nạp được bằng FontFace), không quá lớn. */
export function isSplashCandidate(r: Cand, maxBytes = MAX_FONT_BYTES): boolean {
  return !!r.path && !r.missing && r.faceIndex === 0 && r.fileSize > 0 && r.fileSize <= maxBytes && isSafeFontPath(r.path);
}

/** Tối đa `max` face, mỗi family một lần, xáo ngẫu nhiên (rand ∈ [0,1)). */
export function pickFonts<T extends Cand>(rows: readonly T[], rand: () => number, max = MAX_USER_FONTS): T[] {
  const ok = rows.filter(r => isSplashCandidate(r));
  const a = [...ok];
  for (let i = a.length - 1; i > 0; i--) { const j = Math.floor(rand() * (i + 1)); [a[i], a[j]] = [a[j], a[i]]; }
  const seen = new Set<string>(); const out: T[] = [];
  for (const r of a) { if (seen.has(r.family)) continue; seen.add(r.family); out.push(r); if (out.length >= max) break; }
  return out;
}

export const TICKS = 12;
/** Độ trễ giữa các nhịp: chậm dần (nhân `growth` mỗi nhịp). Tổng ≈ 1,5 s với mặc định. */
export function tickDelays(count = TICKS, firstMs = 45, growth = 1.17): number[] {
  return Array.from({ length: count }, (_, i) => Math.round(firstMs * growth ** i));
}
/** Ký tự i khoá vào font cuối ở nhịp này (trái sang phải, lệch nhịp), nhịp cuối khoá nốt. */
export const lockTick = (i: number, count = TICKS): number => Math.min(count, Math.round(6 + i * 0.75));

/** Chọn một font từ pool, tránh lặp ngay font vừa dùng cho ký tự đó. */
export function pickFrom(pool: readonly PoolFont[], rand: () => number, avoid?: PoolFont): PoolFont {
  const cand = pool.length > 1 && avoid ? pool.filter(p => p !== avoid) : pool;
  return cand[Math.floor(rand() * cand.length)];
}

/** Nhịp `tick` (1…count) cho 9 ký tự: ký tự đã tới lượt khoá → font cuối; còn lại → ngẫu nhiên. */
export type Slot = { font: PoolFont; locked: boolean };
export function finalFont(i: number): PoolFont {
  return i < SERIF_PART.length ? { family: SERIF.family, weight: SERIF.weight, ratio: xRatio(SERIF), user: false } : { family: SANS.family, weight: SANS.weight, ratio: xRatio(SANS), user: false };
}
export function stepSlots(prev: readonly Slot[], tick: number, pool: readonly PoolFont[], rand: () => number, count = TICKS): Slot[] {
  return Array.from({ length: WORD.length }, (_, i) => {
    if (tick >= lockTick(i, count)) return { font: finalFont(i), locked: true };
    return { font: pickFrom(pool, rand, prev[i]?.font), locked: false };
  });
}
