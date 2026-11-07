import type { CompareMetric } from '../../api/compare';

export type Delta = { kind: 'percent' | 'degrees'; value: number };
const DEGREE_METRICS = new Set(['slant_deg', 'stress_deg']);

/** Δ của một cột so với mốc: % cho độ dài/tỷ lệ, độ tuyệt đối cho góc (mốc 0 vẫn có nghĩa). Không đo được → null. */
export function deltaOf(key: string, v: CompareMetric, base: CompareMetric): Delta | null {
  if (v.value === null || base.value === null) return null;
  if (v.delta !== undefined) { // backend đã tính: dùng nguyên, không tự tính lại
    const x = v.delta ? (DEGREE_METRICS.has(key) ? v.delta.abs : v.delta.pct) : null;
    return x === null || x === undefined || !Number.isFinite(x) ? null : { kind: DEGREE_METRICS.has(key) ? 'degrees' : 'percent', value: x };
  }
  if (DEGREE_METRICS.has(key)) return { kind: 'degrees', value: v.value - base.value };
  if (Math.abs(base.value) < 1e-3) return null; // mốc ~0: % vô nghĩa
  return { kind: 'percent', value: (v.value - base.value) / base.value * 100 };
}

const nf = (v: number, digits: number) => new Intl.NumberFormat('vi-VN', { minimumFractionDigits: digits, maximumFractionDigits: digits, useGrouping: false }).format(Math.abs(v));
export function deltaText(d: Delta): { text: string; tone: 'up' | 'down' | 'zero' } {
  const tiny = d.kind === 'percent' ? Math.abs(d.value) < 1 : Math.abs(d.value) < 0.5;
  if (tiny) return { text: '±0', tone: 'zero' };
  const sign = d.value > 0 ? '+' : '−';
  const body = d.kind === 'degrees' ? `${nf(d.value, 1)}°` : `${nf(d.value, Math.abs(d.value) < 10 ? 1 : 0)}%`;
  return { text: `Δ ${sign}${body}`, tone: d.value > 0 ? 'up' : 'down' };
}

export const FIT_MIN = 0.5;
export const FIT_MAX = 2;
export const xhOk = (v: number | null | undefined): v is number => v != null && Number.isFinite(v) && v > 0.1 && v < 1;
/** Hệ số cỡ chữ để x-height của cột bằng mốc (Match x-height), kẹp trong 0,5–2. Thiếu số hoặc số vô lý → 1. */
export function fitScale(baseXh: number | null, xh: number | null): number {
  return xhOk(baseXh) && xhOk(xh) ? Math.min(FIT_MAX, Math.max(FIT_MIN, baseXh / xh)) : 1;
}

export const percentLabel = (p: number | null): string => (p === null ? '' : `P${Math.round(Math.min(100, Math.max(0, p)))}`);
export const rowUnit = (cells: CompareMetric[]): string | null => cells.find(c => c.value !== null)?.unit ?? cells.find(c => c.unit)?.unit ?? null;
