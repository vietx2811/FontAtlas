// Bản mock của phép lọc/thống kê weight–width class và histogram, khớp backend (fontatlas-store + services::analysis).
import type { AxisHistogramDto, FaceRowDto } from '../bindings';

/** usWeightClass chuẩn hoá về 100..=900: 1–9 → ×100, ≤ 0/null → null, ngoài biên kẹp vào biên (như `WEIGHT_NORM` của store). */
export const normWeight = (w: number | null | undefined): number | null => (w == null || w <= 0 ? null : w <= 9 ? w * 100 : Math.min(900, Math.max(100, w)));
/** usWidthClass hợp lệ 1..=9, còn lại null. */
export const normWidth = (w: number | null | undefined): number | null => (w != null && w >= 1 && w <= 9 ? w : null);

/** `wdth` (%) ứng với usWidthClass 1..9 theo OS/2. */
export const WIDTH_PCT = [50, 62.5, 75, 87.5, 100, 112.5, 125, 150, 200];
/** Khoảng `wdth` (%) của các class a..=b; ranh giới là trung điểm giữa hai class liền kề. */
export function widthPctSpan(a: number, b: number): [number, number] {
  const i = Math.min(9, Math.max(1, a)) - 1, j = Math.min(9, Math.max(1, b)) - 1;
  return [i === 0 ? WIDTH_PCT[0] : (WIDTH_PCT[i - 1] + WIDTH_PCT[i]) / 2, j === 8 ? WIDTH_PCT[8] : (WIDTH_PCT[j] + WIDTH_PCT[j + 1]) / 2];
}

export type VarSpan = { wght?: [number, number]; wdth?: [number, number] };
/** Mock không có dữ liệu fvar: font Variable giả định phủ wght 100–900 và wdth 75–125. */
export const mockVarSpan = (r: Pick<FaceRowDto, 'isVariable'>): VarSpan => (r.isVariable ? { wght: [100, 900], wdth: [75, 125] } : {});

type Cls = Pick<FaceRowDto, 'weightClass' | 'widthClass' | 'isVariable'>;

export function matchesWeight(r: Cls, range: [number, number], varMatch: boolean, span: VarSpan = mockVarSpan(r)): boolean {
  const [a, b] = [Math.min(...range), Math.max(...range)];
  const w = normWeight(r.weightClass);
  if (w != null && w >= a && w <= b) return true;
  return varMatch && !!span.wght && span.wght[0] <= b && span.wght[1] >= a;
}

/** Khoảng nửa mở [lo, hi): giá trị đúng ranh giới thuộc class bên trên; class 9 đóng ở 200. */
export function matchesWidth(r: Cls, range: [number, number], varMatch: boolean, span: VarSpan = mockVarSpan(r)): boolean {
  const [a, b] = [Math.max(1, Math.min(...range)), Math.min(9, Math.max(...range))];
  const w = normWidth(r.widthClass);
  if (w != null && w >= a && w <= b) return true;
  if (!varMatch || !span.wdth) return false;
  const [lo, hi] = widthPctSpan(a, b);
  return (b === 9 ? span.wdth[0] <= hi : span.wdth[0] < hi) && span.wdth[1] >= lo;
}

/** Hai chuỗi 9 nấc: `bins` (instance mặc định) và `spanBins` (Variable đếm vào mọi nấc dải trục phủ). */
export function classBins(rows: Cls[], axis: 'weight' | 'width', spanOf: (r: Cls) => VarSpan = mockVarSpan): { bins: number[]; spanBins: number[] } {
  const bins = new Array<number>(9).fill(0), spanBins = new Array<number>(9).fill(0);
  for (const r of rows) {
    const v = axis === 'weight' ? normWeight(r.weightClass) : normWidth(r.widthClass);
    if (v == null) continue;
    const k = axis === 'weight' ? Math.min(9, Math.max(1, Math.round(v / 100))) : v;
    bins[k - 1]++;
    const covered = new Set([k]);
    const span = spanOf(r)[axis === 'weight' ? 'wght' : 'wdth'];
    if (span) for (let c = 1; c <= 9; c++) {
      if (axis === 'weight' ? span[0] <= c * 100 && c * 100 <= span[1] : (() => { const [lo, hi] = widthPctSpan(c, c); return (c === 9 ? span[0] <= hi : span[0] < hi) && span[1] >= lo; })()) covered.add(c);
    }
    covered.forEach(c => { spanBins[c - 1]++; });
  }
  return { bins, spanBins };
}

const quantile = (sorted: number[], p: number) => { if (!sorted.length) return null; const at = (sorted.length - 1) * p; const i = Math.floor(at); return sorted[i] + (sorted[Math.ceil(at)] - sorted[i]) * (at - i); };

/** Bin của v trong [lo, hi] chia nb phần; ngoài miền (kể cả đúng hi) dồn vào bin cuối/đầu. */
export const binOf = (v: number, lo: number, hi: number, nb: number) => Math.min(nb - 1, Math.max(0, Math.floor(((v - lo) / Math.max(hi - lo, Number.EPSILON)) * nb)));

/** Histogram một trục đo: miền = p1–p99 của `domain` (nới ±0.5 khi suy biến), đếm `counted`. */
export function measuredHistogram(axis: string, unit: string, domain: number[], counted: number[], nb: number): AxisHistogramDto {
  const sorted = [...domain].sort((a, b) => a - b);
  let lo = quantile(sorted, 0.01) ?? 0, hi = quantile(sorted, 0.99) ?? 0;
  if (sorted.length && hi <= lo) { hi = lo + 0.5; lo -= 0.5; }
  const bins = new Array<number>(nb).fill(0);
  counted.forEach(v => { bins[binOf(v, lo, hi, nb)]++; });
  return { axis, unit, count: counted.length, lo, hi, bins, spanBins: [...bins], empty: sorted.length === 0 };
}
